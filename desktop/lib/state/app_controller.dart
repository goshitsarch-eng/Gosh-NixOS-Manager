import 'package:flutter/material.dart';

import '../bridge/core_api.dart';
import '../models/configuration.dart';
import '../services/helper_session.dart';
import '../services/preferences.dart';

class AppController extends ChangeNotifier {
  AppController({
    required this.core,
    required this.preferenceStore,
    this.helper,
  });
  final CoreApi core;
  final PreferenceStore preferenceStore;
  HelperApi? helper;
  Preferences preferences = Preferences();
  ConfigurationDto? configuration;
  Map<String, dynamic> catalogs = {};
  Map<String, dynamic> host = {};
  List<Map<String, dynamic>> generations = [];
  Map<String, dynamic>? diskUsage;
  final List<String> logs = [];
  final Map<String, String> fieldErrors = {};
  final Map<String, String> fieldDrafts = {};
  String? error;
  String? busy;
  String preview = '';
  String profilePreview = '';
  String rebuild = 'Switch';
  int page = 0;
  int _revision = 0;
  bool dirty = false;
  bool _disposed = false;
  @override
  void notifyListeners() {
    if (!_disposed) super.notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }

  bool get canManage => host['can_manage'] == true && helper != null;
  bool get ready => configuration != null;
  bool get idle => busy == null;

  List<Map<String, dynamic>> catalog(String name) =>
      List<Map<String, dynamic>>.from(catalogs[name] as List? ?? []);

  Future<void> initialize() async {
    await perform('Opening NixOS Toolkit', () async {
      catalogs = await core.call('bootstrap');
      if (catalogs['api_version'] != 1) {
        throw const CoreFailure('protocol', 'Unsupported core API version');
      }
      configuration = ConfigurationDto(
        catalogs['defaults'] as Map<String, dynamic>,
      );
      try {
        preferences = await preferenceStore.load();
      } catch (failure) {
        error =
            'Preferences could not be read: $failure. The original file is retained.';
      }
      await _refreshHost();
      if (canManage) {
        try {
          final reply = await helper!.request('ReadState');
          if (reply['type'] != 'State') {
            throw const HelperFailure('Invalid saved-state response');
          }
          configuration = ConfigurationDto(
            reply['payload'] as Map<String, dynamic>,
          );
        } catch (failure) {
          error = 'Saved system state could not be loaded: $failure';
        }
      }
      await updatePreview();
    });
  }

  Future<void> _refreshHost() async {
    host = await core.call('probe_host');
    if (host['can_manage'] == true && helper == null) {
      helper = HostHelper.fromCapabilities(host, appendLog);
    }
  }

  Future<void> refresh() =>
      perform('Refreshing host information', _refreshHost);

  Future<void> perform(String title, Future<void> Function() action) async {
    if (!idle) return;
    busy = title;
    notifyListeners();
    try {
      await action();
    } catch (failure) {
      error = failure.toString();
      appendLog('Error: $failure');
    } finally {
      busy = null;
      notifyListeners();
    }
  }

  void appendLog(String line) {
    logs.add(line);
    if (logs.length > 2000) logs.removeRange(0, logs.length - 2000);
    notifyListeners();
  }

  void reportError(String message) {
    error = message;
    notifyListeners();
  }

  void clearError() {
    error = null;
    notifyListeners();
  }

  void setRebuild(String value) {
    rebuild = value;
    notifyListeners();
  }

  void setFieldError(String key, String? message) {
    if (message == null) {
      fieldErrors.remove(key);
    } else {
      fieldErrors[key] = message;
      dirty = true;
      _revision++;
    }
    notifyListeners();
  }

  void selectPage(int index) {
    page = index;
    notifyListeners();
    if (index == 10) updatePreview();
    if (index == 8 && canManage) loadGenerations();
    if (index == 9 && canManage) loadDiskUsage();
  }

  void setValue(String key, dynamic value, [String? section]) {
    configuration!.set(key, value, section);
    if (section == 'hardware_config' && key == 'bluetooth_enabled') {
      configuration!.set('bluetooth_enabled', value);
    }
    dirty = true;
    _revision++;
    notifyListeners();
  }

  void toggleList(String key, String value, bool enabled) {
    final values = configuration!.strings(key);
    if (enabled && !values.contains(value)) values.add(value);
    if (!enabled) values.remove(value);
    setValue(key, values);
  }

  Future<void> selectProfile(String? id) async {
    setValue('selected_profile', id);
    if (id == null) {
      profilePreview = '';
    } else {
      try {
        final response = await core.call('profile_preview', {'id': id});
        if (configuration!.profile == id) {
          profilePreview = response['content'] as String;
        }
      } catch (failure) {
        error = failure.toString();
      }
    }
    notifyListeners();
  }

  void toggleBundle(Map<String, dynamic> bundle, bool enabled) {
    final id = bundle['id'] as String;
    toggleList('enabled_bundles', id, enabled);
    final packages = Map<String, dynamic>.from(
      configuration!.value('bundle_packages') as Map,
    );
    if (enabled) {
      packages[id] = (bundle['packages'] as List)
          .map((p) => (p as Map)['id'])
          .toList();
    } else {
      packages.remove(id);
    }
    setValue('bundle_packages', packages);
  }

  void toggleBundlePackage(String id, String package, bool enabled) {
    final selections = Map<String, dynamic>.from(
      configuration!.value('bundle_packages') as Map,
    );
    final bundle = catalog('bundles').firstWhere((b) => b['id'] == id);
    final current = List<String>.from(
      selections[id] as List? ??
          (bundle['packages'] as List).map((p) => (p as Map)['id']).toList(),
    );
    if (enabled && !current.contains(package)) current.add(package);
    if (!enabled) current.remove(package);
    selections[id] = current;
    setValue('bundle_packages', selections);
  }

  Future<String> addPackages(String input) async {
    final parsed = await core.call('parse_packages', {
      'input': input,
      'existing': configuration!.strings('custom_packages'),
    });
    final added = List<String>.from(parsed['added'] as List);
    if (added.isNotEmpty) {
      setValue('custom_packages', [
        ...configuration!.strings('custom_packages'),
        ...added,
      ]);
    }
    final bundled = parsed['bundled'] as List;
    return [
      if (added.isNotEmpty) 'Added ${added.join(', ')}',
      if ((parsed['duplicates'] as List).isNotEmpty)
        'Already added: ${(parsed['duplicates'] as List).join(', ')}',
      if (bundled.isNotEmpty)
        'Available in bundles: ${bundled.map((p) => '${p[0]} (${p[1]})').join(', ')}',
    ].join('\n');
  }

  Future<void> updatePreview() async {
    if (!ready) return;
    final revision = _revision;
    try {
      final response = await core.call('preview', {
        'state': configuration!.toJson(),
      });
      if (revision == _revision) preview = response['content'] as String;
    } catch (failure) {
      if (revision == _revision) {
        preview = 'Configuration needs attention: $failure';
      }
    }
    notifyListeners();
  }

  Future<void> setTheme(ThemeMode mode) async {
    preferences.themeMode = mode;
    notifyListeners();
    try {
      await preferenceStore.save(preferences);
    } catch (failure) {
      error = 'Could not save preferences: $failure';
      notifyListeners();
    }
  }

  Future<void> apply({bool dryRun = false}) => perform(
    dryRun ? 'Checking configuration' : 'Applying configuration',
    () async {
      if (!canManage) {
        throw const HelperFailure(
          'A NixOS host and installed privileged helper are required',
        );
      }
      if (fieldErrors.isNotEmpty) {
        throw const CoreFailure(
          'validation',
          'Correct invalid fields before applying',
        );
      }
      final snapshot = configuration!.copy();
      final revision = _revision;
      await core.call('validate', {'state': snapshot.toJson()});
      await helper!.apply(snapshot.toJson(), dryRun ? 'DryBuild' : rebuild);
      if (!dryRun && revision == _revision) dirty = false;
      appendLog(
        dryRun
            ? 'Dry build completed; managed files restored'
            : 'Changes applied and state saved',
      );
    },
  );

  Future<void> loadGenerations() => perform('Loading generations', () async {
    if (!canManage) return;
    final response = await helper!.request('ListGenerations');
    if (response['type'] != 'Generations') {
      throw const HelperFailure('Invalid generation response');
    }
    generations = List<Map<String, dynamic>>.from(response['payload'] as List);
  });

  Future<void> generationAction(
    int number, {
    String? activation,
    bool delete = false,
  }) => perform(
    delete ? 'Deleting generation' : 'Activating generation',
    () async {
      if (!canManage) throw const HelperFailure('Host helper is unavailable');
      await helper!.request(
        delete ? 'DeleteGenerations' : 'RollbackGeneration',
        delete
            ? {
                'generations': [number],
              }
            : {'generation': number, 'activate': activation ?? 'switch'},
      );
      final response = await helper!.request('ListGenerations');
      generations = List<Map<String, dynamic>>.from(
        response['payload'] as List,
      );
    },
  );

  Future<void> loadDiskUsage() => perform('Reading store usage', () async {
    if (!canManage) return;
    final response = await helper!.request('GetDiskUsage');
    if (response['type'] != 'DiskUsage') {
      throw const HelperFailure('Invalid disk usage response');
    }
    diskUsage = response['payload'] as Map<String, dynamic>;
  });

  Future<void> maintenance(String command) =>
      perform('Running maintenance', () async {
        if (!canManage) throw const HelperFailure('Host helper is unavailable');
        final response = await helper!.request('RunMaintenance', {
          'command': command,
        });
        final result = response['payload'] as Map<String, dynamic>;
        appendLog('${result['stdout']}\n${result['stderr']}');
        if (response['type'] != 'MaintenanceOutput' ||
            result['success'] != true) {
          throw const HelperFailure('Maintenance failed; review the log');
        }
      });
}

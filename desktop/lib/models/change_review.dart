import 'dart:convert';

import 'configuration.dart';

/// A review of requested Toolkit changes, not an inventory of installed apps.
class ChangeReview {
  ChangeReview(this.before, this.after, this.catalogs);
  final ConfigurationDto before;
  final ConfigurationDto after;
  final Map<String, dynamic> catalogs;

  Iterable<Map> get _bundles =>
      (catalogs['bundles'] as List? ?? []).cast<Map>();
  Iterable<Map> get _packages =>
      _bundles.expand((bundle) => (bundle['packages'] as List).cast<Map>());

  Map<String, String> _software(ConfigurationDto state) {
    final definitions = {
      for (final package in _packages) package['id']: package,
    };
    final selected = state.value('bundle_packages') as Map;
    final ids = <String>{...state.strings('custom_packages')};
    for (final bundle in _bundles) {
      if (!state.strings('enabled_bundles').contains(bundle['id'])) continue;
      ids.addAll(
        List<String>.from(
          selected[bundle['id']] as List? ??
              (bundle['packages'] as List)
                  .cast<Map>()
                  .where((p) => p['unavailable_reason'] == null)
                  .map((p) => p['id'])
                  .toList(),
        ),
      );
    }
    // Collapse aliases such as npm/JavaScript/Node.js in the same review.
    final result = <String, String>{};
    for (final id in ids.toList()..sort()) {
      final package = definitions[id];
      final attr = package?['nix_attr'] as String? ?? id;
      result.putIfAbsent(attr, () => package?['display_name'] as String? ?? id);
    }
    return result;
  }

  List<String> get additions {
    final old = _software(before), current = _software(after);
    return [
      for (final id in current.keys)
        if (!old.containsKey(id)) current[id]!,
    ]..sort();
  }

  List<String> get removals {
    final old = _software(before), current = _software(after);
    return [
      for (final id in old.keys)
        if (!current.containsKey(id)) old[id]!,
    ]..sort();
  }

  String _name(String catalog, String? id) {
    if (id == null) return 'No Toolkit selection';
    for (final entry in (catalogs[catalog] as List? ?? []).cast<Map>()) {
      if (entry['id'] == id) return entry['name'] as String;
    }
    return id;
  }

  static String _label(String key) {
    const labels = {
      'hostname': 'Hostname',
      'dns_servers': 'DNS servers',
      'username': 'User for group membership',
      'user_groups': 'User groups',
      'allowed_tcp_ports': 'Open TCP ports',
      'allowed_udp_ports': 'Open UDP ports',
      'ssh_enabled': 'SSH server',
      'ssh_port': 'SSH port',
      'ssh_password_auth': 'SSH password login',
      'ssh_root_login': 'SSH root login',
      'wireguard_listen_port': 'WireGuard port',
      'audio_server': 'Audio system',
      'nvidia_driver': 'NVIDIA driver',
      'nvidia_open': 'Open NVIDIA kernel modules',
      'auto_upgrade': 'Automatic upgrades',
      'auto_gc': 'Automatic garbage collection',
      'libvirtd': 'Virtual machines (libvirt)',
      'resolved': 'DNS resolver',
      'networkmanager': 'NetworkManager',
      'store_optimize': 'Store optimization',
    };
    return labels[key] ??
        key
            .split('_')
            .map(
              (word) => word.isEmpty
                  ? word
                  : '${word[0].toUpperCase()}${word.substring(1)}',
            )
            .join(' ');
  }

  static String _value(String key, dynamic value) {
    if (value == null || value == '' || value is List && value.isEmpty) {
      return 'Not set';
    }
    if (value is bool) return value ? 'On' : 'Off';
    if (value is List) return value.join(', ');
    final choices = switch (key) {
      'nvidia_driver' => ['Stable', 'Beta', 'Open', 'Nouveau'],
      'audio_server' => ['PipeWire', 'PulseAudio', 'Disabled'],
      'power_profile' => ['Balanced', 'Performance', 'Power saver'],
      _ => <String>[],
    };
    if (value is int && value >= 0 && value < choices.length) {
      return choices[value];
    }
    return value.toString();
  }

  List<String> get settings {
    final changes = <String>[];
    if (before.profile != after.profile) {
      changes.add(
        'Desktop: ${_name('profiles', before.profile)} → ${_name('profiles', after.profile)}',
      );
    }
    final oldBundles = before.strings('enabled_bundles').toSet();
    final newBundles = after.strings('enabled_bundles').toSet();
    for (final id in newBundles.difference(oldBundles)) {
      changes.add('Enable bundle: ${_name('bundles', id)}');
    }
    for (final id in oldBundles.difference(newBundles)) {
      changes.add('Stop managing bundle: ${_name('bundles', id)}');
    }
    for (final key in ['hostname', 'dns_servers', 'username', 'user_groups']) {
      if (jsonEncode(before.value(key)) != jsonEncode(after.value(key))) {
        changes.add(
          '${_label(key)}: ${_value(key, before.value(key))} → ${_value(key, after.value(key))}',
        );
      }
    }
    for (final section in [
      'network_config',
      'hardware_config',
      'services_config',
    ]) {
      final old = before.value(section) as Map,
          current = after.value(section) as Map;
      for (final key in {
        ...old.keys,
        ...current.keys,
      }.cast<String>().toList()..sort()) {
        if (jsonEncode(old[key]) == jsonEncode(current[key])) continue;
        if (section == 'services_config') {
          changes.add(
            '${_label(key)}: ${current[key] == true ? 'Request service' : 'Stop requesting service'}',
          );
        } else {
          changes.add(
            '${_label(key)}: ${_value(key, old[key])} → ${_value(key, current[key])}',
          );
        }
      }
    }
    return changes;
  }

  List<String> get notes => [
    if (before.profile != after.profile)
      'A desktop change can replace your login screen and desktop session.',
    if (jsonEncode(before.value('network_config')) !=
        jsonEncode(after.value('network_config')))
      'Network changes can interrupt a remote connection.',
    if (jsonEncode(before.value('hardware_config')) !=
        jsonEncode(after.value('hardware_config')))
      'Hardware changes regenerate the hardware module, including its selected audio and power settings. Review those settings even if you changed only Bluetooth or a driver.',
    if (settings.any((line) => line.startsWith('Enable bundle:'))) 'Bundles can also configure system services. Review the generated configuration below.',
    if (removals.isNotEmpty) 'Software requested elsewhere in NixOS, Home Manager or another installer will remain managed there.',
  ];

  String get text => [
    if (additions.isNotEmpty)
      'Add software selections (${additions.length}): ${additions.join(', ')}',
    if (removals.isNotEmpty)
      'Remove software selections (${removals.length}): ${removals.join(', ')}',
    ...settings,
    if (additions.isEmpty && removals.isEmpty && settings.isEmpty) 'No package or system-setting changes in this review. Rebuilding can still use updated packages or other NixOS configuration.',
    ...notes,
  ].join('\n\n');
}

String rebuildExplanation(String mode) => switch (mode) {
  'Build' => 'Build and save the proposed configuration without activating it or changing the next boot. A later NixOS rebuild can activate these saved settings.',
  'Boot' => 'Build and select this configuration for the next restart. Keep the current running system until then.',
  'Test' => 'Activate this configuration now without changing the next boot. This can restart services; it does not automatically revert after a timeout.',
  _ => 'Activate this configuration now and select it for future boots. Running services may restart.',
};

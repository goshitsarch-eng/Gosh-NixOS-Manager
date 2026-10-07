import 'package:flutter/material.dart';

import '../../state/app_controller.dart';
import '../components/settings.dart';

class SystemScreen extends StatelessWidget {
  const SystemScreen(this.controller, {super.key});
  final AppController controller;
  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      SettingsSection(
        title: 'Appearance',
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: SegmentedButton<ThemeMode>(
              segments: const [
                ButtonSegment(
                  value: ThemeMode.system,
                  icon: Icon(Icons.brightness_auto),
                  label: Text('System'),
                ),
                ButtonSegment(
                  value: ThemeMode.light,
                  icon: Icon(Icons.light_mode),
                  label: Text('Light'),
                ),
                ButtonSegment(
                  value: ThemeMode.dark,
                  icon: Icon(Icons.dark_mode),
                  label: Text('Dark'),
                ),
              ],
              selected: {controller.preferences.themeMode},
              onSelectionChanged: (values) => controller.setTheme(values.first),
            ),
          ),
        ],
      ),
      SettingsSection(
        title: 'Network identity',
        description: 'Explicit values in your existing Nix configuration take precedence.',
        children: [
          SettingText(
            controller: controller,
            field: 'hostname',
            label: 'Hostname',
            hint: controller.host['system']?['hostname'] as String?,
          ),
          SettingText(
            controller: controller,
            field: 'dns_servers',
            label: 'DNS servers (IPv4)',
            hint: '1.1.1.1, 8.8.8.8',
            list: true,
          ),
        ],
      ),
      SettingsSection(
        title: 'User groups',
        description: 'Choose a username and the groups it should join.',
        children: [
          SettingText(
            controller: controller,
            field: 'username',
            label: 'Username',
          ),
          ...{
            'libvirtd': 'Virtual machine management',
            'docker': 'Docker',
            'vboxusers': 'VirtualBox',
          }.entries.map(
            (group) => CheckboxListTile(
              title: Text(group.value),
              subtitle: Text(group.key),
              value: controller.configuration!
                  .strings('user_groups')
                  .contains(group.key),
              onChanged: controller.idle
                  ? (value) =>
                        controller.toggleList('user_groups', group.key, value!)
                  : null,
            ),
          ),
        ],
      ),
    ],
  );
}

class HardwareScreen extends StatelessWidget {
  const HardwareScreen(this.controller, {super.key});
  final AppController controller;
  @override
  Widget build(BuildContext context) {
    final isArm = controller.host['arch'] == 'Aarch64';
    final nvidia =
        !isArm && '${controller.host['gpu']}'.toLowerCase().contains('nvidia');
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SettingsSection(
          title: 'Graphics',
          children: [
            ListTile(
              leading: const Icon(Icons.memory),
              title: Text(
                '${controller.host['gpu'] ?? 'GPU detection unavailable'}',
              ),
            ),
            if (nvidia) ...[
              SettingChoice(
                controller: controller,
                field: 'nvidia_driver',
                section: 'hardware_config',
                label: 'NVIDIA driver',
                values: const {
                  null: 'Keep existing',
                  0: 'Stable',
                  1: 'Beta',
                  2: 'Open',
                  3: 'Nouveau',
                },
              ),
              ...{
                'nvidia_modesetting': 'Kernel modesetting',
                'nvidia_powermanagement': 'NVIDIA power management',
                'nvidia_open': 'Open kernel modules',
              }.entries.map(
                (e) => SettingSwitch(
                  controller: controller,
                  field: e.key,
                  label: e.value,
                  section: 'hardware_config',
                ),
              ),
            ],
          ],
        ),
        SettingsSection(
          title: 'Audio',
          children: [
            SettingChoice(
              controller: controller,
              field: 'audio_server',
              section: 'hardware_config',
              label: 'Audio server',
              values: const {0: 'PipeWire', 1: 'PulseAudio', 2: 'None'},
            ),
            SettingSwitch(
              controller: controller,
              field: 'audio_lowlatency',
              label: 'Low latency audio',
              section: 'hardware_config',
              enabled:
                  controller.configuration!.value(
                    'audio_server',
                    'hardware_config',
                  ) ==
                  0,
            ),
          ],
        ),
        SettingsSection(
          title: 'Bluetooth',
          children: [
            SettingSwitch(
              controller: controller,
              field: 'bluetooth_enabled',
              label: 'Enable Bluetooth',
              section: 'hardware_config',
            ),
            SettingSwitch(
              controller: controller,
              field: 'bluetooth_autopower',
              label: 'Power on at boot',
              section: 'hardware_config',
            ),
          ],
        ),
        SettingsSection(
          title: 'Power management',
          children: [
            SettingChoice(
              controller: controller,
              field: 'power_profile',
              section: 'hardware_config',
              label: 'Power profile',
              values: const {0: 'Balanced', 1: 'Performance', 2: 'Power saver'},
            ),
            SettingSwitch(
              controller: controller,
              field: 'tlp_enabled',
              label: 'TLP battery management',
              section: 'hardware_config',
            ),
            SettingSwitch(
              controller: controller,
              field: 'thermald_enabled',
              label: 'Intel thermald',
              section: 'hardware_config',
              enabled: !isArm,
              description: isArm ? 'Unavailable on ARM' : null,
            ),
          ],
        ),
      ],
    );
  }
}

class NetworkScreen extends StatelessWidget {
  const NetworkScreen(this.controller, {super.key});
  final AppController controller;
  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      SettingsSection(
        title: 'Firewall',
        children: [
          SettingSwitch(
            controller: controller,
            field: 'firewall_enabled',
            label: 'Enable firewall',
            section: 'network_config',
          ),
          SettingText(
            controller: controller,
            field: 'allowed_tcp_ports',
            label: 'Allowed TCP ports',
            section: 'network_config',
            ports: true,
          ),
          PortPresets(controller, 'allowed_tcp_ports', const [
            22,
            80,
            443,
            8080,
          ]),
          SettingText(
            controller: controller,
            field: 'allowed_udp_ports',
            label: 'Allowed UDP ports',
            section: 'network_config',
            ports: true,
          ),
          PortPresets(controller, 'allowed_udp_ports', const [
            53,
            123,
            443,
            51820,
          ]),
        ],
      ),
      SettingsSection(
        title: 'Secure Shell',
        children: [
          SettingSwitch(
            controller: controller,
            field: 'ssh_enabled',
            label: 'Enable SSH',
            section: 'network_config',
          ),
          SettingText(
            controller: controller,
            field: 'ssh_port',
            label: 'SSH port',
            section: 'network_config',
            number: true,
          ),
          SettingSwitch(
            controller: controller,
            field: 'ssh_password_auth',
            label: 'Password authentication',
            section: 'network_config',
          ),
          SettingChoice(
            controller: controller,
            field: 'ssh_root_login',
            label: 'Root login',
            section: 'network_config',
            values: const {
              'no': 'Disabled',
              'prohibit-password': 'Keys only',
              'yes': 'Allowed',
            },
          ),
          SettingSwitch(
            controller: controller,
            field: 'fail2ban_enabled',
            label: 'Fail2Ban protection',
            section: 'network_config',
          ),
        ],
      ),
      SettingsSection(
        title: 'VPN',
        description: 'WireGuard enables the module and opens its listen port. Configure peers and keys in your Nix configuration.',
        children: [
          SettingSwitch(
            controller: controller,
            field: 'tailscale_enabled',
            label: 'Enable Tailscale',
            section: 'network_config',
          ),
          SettingSwitch(
            controller: controller,
            field: 'wireguard_enabled',
            label: 'Enable WireGuard',
            section: 'network_config',
          ),
          SettingText(
            controller: controller,
            field: 'wireguard_listen_port',
            label: 'WireGuard listen port',
            section: 'network_config',
            number: true,
          ),
        ],
      ),
    ],
  );
}

const serviceGroups = <String, Map<String, String>>{
  'Hardware': {
    'printing': 'Printing (CUPS)',
    'avahi': 'Avahi / mDNS',
    'fwupd': 'Firmware updates',
    'upower': 'UPower',
  },
  'Network': {
    'networkmanager': 'NetworkManager',
    'resolved': 'systemd-resolved',
  },
  'Remote and synchronization': {
    'rustdesk': 'RustDesk client',
    'syncthing': 'Syncthing',
    'locate': 'Locate (plocate)',
  },
  'Desktop': {
    'flatpak': 'Flatpak support',
    'gnome_keyring': 'GNOME Keyring',
    'gnome_tweaks': 'GNOME Tweaks package',
    'dconf': 'dconf',
  },
  'Development': {
    'docker': 'Docker',
    'libvirtd': 'libvirtd',
    'postgresql': 'PostgreSQL',
    'redis': 'Redis',
  },
  'System': {
    'earlyoom': 'Early OOM',
    'auto_upgrade': 'Automatic upgrades',
    'auto_gc': 'Automatic garbage collection',
    'store_optimize': 'Store optimization',
  },
};

class ServicesScreen extends StatelessWidget {
  const ServicesScreen(this.controller, {super.key});
  final AppController controller;
  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: serviceGroups.entries
        .map(
          (group) => SettingsSection(
            title: group.key,
            children: group.value.entries
                .map(
                  (service) => SettingSwitch(
                    controller: controller,
                    field: service.key,
                    label: service.value,
                    section: 'services_config',
                    description: service.key == 'rustdesk'
                        ? 'Installs the client package, not a remote desktop server'
                        : null,
                  ),
                )
                .toList(),
          ),
        )
        .toList(),
  );
}

class PortPresets extends StatelessWidget {
  const PortPresets(this.controller, this.field, this.ports, {super.key});
  final AppController controller;
  final String field;
  final List<int> ports;
  @override
  Widget build(BuildContext context) {
    final selected = List<int>.from(
      controller.configuration!.value(field, 'network_config'),
    );
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
      child: Wrap(
        spacing: 8,
        children: ports
            .map(
              (port) => FilterChip(
                label: Text('$port'),
                selected: selected.contains(port),
                onSelected: controller.idle
                    ? (enabled) {
                        final next = [...selected];
                        if (enabled) {
                          next.add(port);
                        } else {
                          next.remove(port);
                        }
                        controller.fieldDrafts.remove('network_config.$field');
                        controller.setFieldError('network_config.$field', null);
                        controller.setValue(
                          field,
                          next.toSet().toList(),
                          'network_config',
                        );
                      }
                    : null,
              ),
            )
            .toList(),
      ),
    );
  }
}

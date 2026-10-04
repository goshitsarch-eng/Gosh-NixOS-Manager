import 'dart:convert';

/// Compatibility DTO for the existing Rust AppState and helper JSON protocol.
/// Parsing, validation and configuration generation remain in Rust.
class ConfigurationDto {
  ConfigurationDto(Map<String, dynamic> source)
    : _data = jsonDecode(jsonEncode(source)) as Map<String, dynamic>;
  final Map<String, dynamic> _data;

  dynamic value(String key, [String? section]) => section == null
      ? _data[key]
      : (_data[section] as Map<String, dynamic>)[key];
  void set(String key, dynamic value, [String? section]) {
    if (section == null) {
      _data[key] = value;
    } else {
      (_data[section] as Map<String, dynamic>)[key] = value;
    }
  }

  List<String> strings(String key) => List<String>.from(_data[key] as List);
  Map<String, dynamic> toJson() =>
      jsonDecode(jsonEncode(_data)) as Map<String, dynamic>;
  ConfigurationDto copy() => ConfigurationDto(toJson());
  String? get profile => _data['selected_profile'] as String?;
}

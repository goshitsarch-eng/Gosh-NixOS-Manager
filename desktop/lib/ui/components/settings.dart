import 'package:flutter/material.dart';

import '../../state/app_controller.dart';

class SettingsSection extends StatelessWidget {
  const SettingsSection({
    super.key,
    required this.title,
    required this.children,
    this.description,
  });
  final String title;
  final String? description;
  final List<Widget> children;
  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 24),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(title, style: Theme.of(context).textTheme.titleMedium),
        if (description != null)
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(description!),
          ),
        const SizedBox(height: 10),
        Card(
          margin: EdgeInsets.zero,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: children,
          ),
        ),
      ],
    ),
  );
}

class SettingSwitch extends StatelessWidget {
  const SettingSwitch({
    super.key,
    required this.controller,
    required this.field,
    required this.label,
    required this.section,
    this.description,
    this.enabled = true,
  });
  final AppController controller;
  final String field;
  final String label;
  final String section;
  final String? description;
  final bool enabled;
  @override
  Widget build(BuildContext context) => SwitchListTile(
    title: Text(label),
    subtitle: description == null ? null : Text(description!),
    value: controller.configuration!.value(field, section) == true,
    onChanged: enabled && controller.idle
        ? (value) => controller.setValue(field, value, section)
        : null,
  );
}

class SettingChoice extends StatelessWidget {
  const SettingChoice({
    super.key,
    required this.controller,
    required this.field,
    required this.label,
    required this.values,
    this.section,
    this.description,
    this.enabled = true,
  });
  final AppController controller;
  final String field;
  final String label;
  final String? section;
  final String? description;
  final Map<dynamic, String> values;
  final bool enabled;
  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.all(16),
    child: DropdownButtonFormField<dynamic>(
      key: ValueKey('${section ?? ''}.$field'),
      initialValue: controller.configuration!.value(field, section),
      isExpanded: true,
      decoration: InputDecoration(labelText: label, helperText: description),
      items: values.entries
          .map(
            (e) =>
                DropdownMenuItem<dynamic>(value: e.key, child: Text(e.value)),
          )
          .toList(),
      onChanged: enabled && controller.idle
          ? (value) => controller.setValue(field, value, section)
          : null,
    ),
  );
}

class SettingText extends StatefulWidget {
  const SettingText({
    super.key,
    required this.controller,
    required this.field,
    required this.label,
    this.section,
    this.hint,
    this.list = false,
    this.number = false,
    this.ports = false,
  });
  final AppController controller;
  final String field;
  final String label;
  final String? section;
  final String? hint;
  final bool list;
  final bool number;
  final bool ports;
  @override
  State<SettingText> createState() => _SettingTextState();
}

class _SettingTextState extends State<SettingText> {
  late final TextEditingController text;
  String? error;
  String get errorKey => '${widget.section ?? ''}.${widget.field}';
  @override
  void initState() {
    super.initState();
    final value = widget.controller.configuration!.value(
      widget.field,
      widget.section,
    );
    text = TextEditingController(
      text:
          widget.controller.fieldDrafts[errorKey] ??
          (value is List ? value.join(', ') : value?.toString() ?? ''),
    );
    error = widget.controller.fieldErrors[errorKey];
  }

  @override
  void didUpdateWidget(covariant SettingText oldWidget) {
    super.didUpdateWidget(oldWidget);
    final value = widget.controller.configuration!.value(
      widget.field,
      widget.section,
    );
    final draft =
        widget.controller.fieldDrafts[errorKey] ??
        (value is List ? value.join(', ') : value?.toString() ?? '');
    if (text.text != draft) text.text = draft;
    error = widget.controller.fieldErrors[errorKey];
  }

  @override
  void dispose() {
    text.dispose();
    super.dispose();
  }

  void changed(String raw) {
    widget.controller.fieldDrafts[errorKey] = raw;
    dynamic value = raw.trim();
    String? invalid;
    if (widget.ports) {
      final tokens = raw.split(RegExp(r'[,\s]+')).where((s) => s.isNotEmpty);
      final numbers = tokens.map(int.tryParse).toList();
      if (numbers.any((p) => p == null || p < 1 || p > 65535)) {
        invalid = 'Use ports from 1 to 65535, separated by commas';
      } else {
        value = numbers.cast<int>().toSet().toList();
      }
    } else if (widget.number) {
      value = int.tryParse(raw);
      if (value == null || value < 1 || value > 65535) {
        invalid = 'Enter a port from 1 to 65535';
      }
    } else if (widget.list) {
      value = raw.split(RegExp(r'[,\s]+')).where((s) => s.isNotEmpty).toList();
    } else if (raw.trim().isEmpty) {
      value = null;
    }
    setState(() => error = invalid);
    widget.controller.setFieldError(errorKey, invalid);
    if (invalid == null) {
      widget.controller.setValue(widget.field, value, widget.section);
    }
  }

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.all(16),
    child: TextField(
      key: Key('field-$errorKey'),
      controller: text,
      enabled: widget.controller.idle,
      keyboardType: widget.number ? TextInputType.number : TextInputType.text,
      decoration: InputDecoration(
        labelText: widget.label,
        hintText: widget.hint,
        errorText: error,
      ),
      onChanged: changed,
    ),
  );
}

class CodePanel extends StatelessWidget {
  const CodePanel(this.content, {super.key, this.height = 280});
  final String content;
  final double height;
  @override
  Widget build(BuildContext context) => Container(
    height: height,
    padding: const EdgeInsets.all(16),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainerHigh,
      borderRadius: BorderRadius.circular(10),
    ),
    child: SingleChildScrollView(
      child: SelectableText(
        content,
        style: const TextStyle(
          fontFamily: 'monospace',
          fontSize: 13,
          height: 1.5,
        ),
      ),
    ),
  );
}

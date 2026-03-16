## Translation Files

`locales` stores the UI translation files.

Rules:

- Use the original Chinese text as the JSON key.
- Use the translated text as the JSON value.
- File names use the language code, for example `en.json`, `ja.json`, `ko.json`.
- Chinese is the source language, so it does not need a separate file.

Example:

```json
{
  "剪贴板": "Clipboard",
  "设置": "Settings"
}
```

Loading order:

1. Detect the system UI language automatically.
2. Try `locales/<language>.json`.
3. Fall back to `en.json`.
4. Fall back to the original Chinese text if no translation exists.

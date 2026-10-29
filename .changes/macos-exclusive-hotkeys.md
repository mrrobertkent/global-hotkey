---
"global-hotkey": minor
---

Add `GlobalHotKeyManager::register_with_options` and `RegisterOptions`. On macOS, `RegisterOptions::exclusive` registers the hotkey with `kEventHotKeyExclusive`, so a hotkey that another app already holds exclusively is refused instead of silently never firing. This has no effect on Windows and X11, where a taken hotkey is always refused.

# Learnings: native-qa

## Step 1

- The reply's wire shape is `{"png": <base64>, "width", "height"}`, built by
  the `pub` helper `handpic_backend::mcp::screenshot_reply(png, width,
  height)`; Step 2's native side should answer through it rather than
  building the JSON itself.
- `with_screenshot` adds `screenshot_router` to whatever tools the server
  already lists, so it composes with `with_all_tools` in either order
  (`Mcp::new(..).with_screenshot()` lists eight tools). The native app should
  call it after `with_all_tools`, which replaces the router.
- The tool uses `Bridge::call` (the 5 s `BRIDGE_TIMEOUT`); Step 2 measures
  whether that is enough.
- A Python heredoc turned `b"\x89PNG"` into a raw 0x89 byte, which rustc
  refuses (byte literal non-ASCII, then invalid UTF-8). The tests use plain
  ASCII bytes; the content of the PNG is not what they check.

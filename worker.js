const params = new URLSearchParams(self.location.search)
const scriptName = params.get("script") || "./williamify.js"

try {
  const williamifyModule = await import(scriptName)
  const wasmName = scriptName.replace(".js", "_bg.wasm")

  await williamifyModule.default(wasmName)
} catch (e) {
  console.error("worker failed to initialize:", e)
  throw e
}

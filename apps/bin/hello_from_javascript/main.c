#include <stdio.h>
#include <unistd.h>
#include "quickjs.h"
#include "quickjs-libc.h"

const char script[] = {
#embed "app.js"
};

static JSContext *JS_NewCustomContext(JSRuntime *rt)
{
  JSContext *ctx = JS_NewContext(rt);
  if (!ctx)
    return NULL;
  return ctx;
}

JSRuntime *rt;
JSContext *ctx;
uint8_t *bytecode;
size_t bytecode_len;

__attribute__((export_name("wizer.initialize")))
void wizer_initialize(void) {
    puts("initializing quickjs...");
    rt = JS_NewRuntime();
    js_std_set_worker_new_context_func(JS_NewCustomContext);
    js_std_init_handlers(rt);
    JS_SetModuleLoaderFunc(rt, NULL, js_module_loader, NULL);
    ctx = JS_NewCustomContext(rt);

    char *argv[] = {
        NULL
    };
    js_std_add_helpers(ctx, 0, argv);

    puts("compiling JavaScript sources...");
    JSValue compiled_module = JS_Eval(ctx, script, sizeof(script), "app.js", JS_EVAL_FLAG_COMPILE_ONLY);
    if (JS_IsException(compiled_module)) {
        js_std_dump_error(ctx);
        JS_FreeValue(ctx, compiled_module);
        exit(1);
    }

    puts("writing bytecode into memory...");
    bytecode = JS_WriteObject(ctx, &bytecode_len, compiled_module, JS_WRITE_OBJ_BYTECODE);
    if (!bytecode) {
        js_std_dump_error(ctx);
        exit(1);
    }
}

__attribute__((export_name("wizer.resume")))
void wizer_resume(void) {
    js_std_eval_binary(ctx, bytecode, bytecode_len, JS_EVAL_FLAG_STRICT);

    puts("ready");
    int r = js_std_loop(ctx);
    if (r) {
      js_std_dump_error(ctx);
    }
    // js_std_free_handlers(rt);
    // JS_FreeContext(ctx);
    // JS_FreeRuntime(rt);
}

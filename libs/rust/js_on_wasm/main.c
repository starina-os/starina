#include <stdio.h>
#include <unistd.h>
#include "quickjs.h"
#include "quickjs-libc.h"

const char *APP_SCRIPT = "console.log('Hello from JS:', 40+2)\n";

static JSContext *JS_NewCustomContext(JSRuntime *rt)
{
  JSContext *ctx = JS_NewContext(rt);
  if (!ctx)
    return NULL;
  return ctx;
}

// clang --target=wasm32-wasi --sysroot=/opt/homebrew/opt/wasi-libc/share/wasi-sysroot -L libclang_rt.builtins-wasm32-wasi-24.0/ -lc -DQJS_BUILD_LIBC quickjs-amalgam.c main.c -o app.stage0.wasm -O2 -D_WASI_EMULATED_SIGNAL -lwasi-emulated-signal -D_GNU_SOURCE -lclang_rt.builtins-wasm32 -nodefaultlibs && wizer --allow-wasi -r _start=wizer.resume app.stage0.wasm -o app.stage1.wasm && wasm-opt -O3 app.stage1.wasm -o app.optimized.wasm

JSRuntime *rt;
JSContext *ctx;
JSValue val;

bool initialized = false;

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
    initialized = true;
}

__attribute__((export_name("wizer.resume")))
int wizer_resume(void) {
    puts("eval...");
    JS_Eval(ctx, APP_SCRIPT, strlen(APP_SCRIPT), "app.js", JS_EVAL_FLAG_STRICT);

    puts("looping...");
    int r = js_std_loop(ctx);
    if (r) {
      js_std_dump_error(ctx);
    }
    js_std_free_handlers(rt);
    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);


    return 0;
}

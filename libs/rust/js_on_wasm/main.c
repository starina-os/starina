#include <stdio.h>
#include <unistd.h>
#include "quickjs.h"
#include "quickjs-libc.h"

#include "app.bytecode.h"

static JSContext *JS_NewCustomContext(JSRuntime *rt)
{
  JSContext *ctx = JS_NewContext(rt);
  if (!ctx)
    return NULL;
  return ctx;
}

// $ clang --target=wasm32-wasi --sysroot=/opt/homebrew/opt/wasi-libc/share/wasi-sysroot -L libclang_rt.builtins-wasm32-wasi-24.0/ -lc -DQJS_BUILD_LIBC quickjs-amalgam.c main.c -o app.wasm -O2 -D_WASI_EMULATED_SIGNAL -lwasi-emulated-signal -D_GNU_SOURCE -lclang_rt.builtins-wasm32 -nodefaultlibs

int main(void) {
    JSRuntime *rt;
    JSContext *ctx;
    JSValue val;

    puts("starting quickjs...\n");

    rt = JS_NewRuntime();
    js_std_set_worker_new_context_func(JS_NewCustomContext);
    js_std_init_handlers(rt);
    JS_SetModuleLoaderFunc(rt, NULL, js_module_loader, NULL);
    ctx = JS_NewCustomContext(rt);

    char *argv[] = {
        NULL
    };
    js_std_add_helpers(ctx, 0, argv);
    js_std_eval_binary(ctx, qjsc_app, qjsc_app_size, 0);
    int r = js_std_loop(ctx);
    if (r) {
      js_std_dump_error(ctx);
    }
    js_std_free_handlers(rt);
    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);


    return 0;
}

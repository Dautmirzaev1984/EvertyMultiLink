package main

/*
#include <jni.h>
#include <stdlib.h>

static const char* getStringChars(JNIEnv *env, jstring str) {
    if (str == NULL) return NULL;
    return (*env)->GetStringUTFChars(env, str, NULL);
}

static void releaseStringChars(JNIEnv *env, jstring str, const char *chars) {
    if (chars != NULL) {
        (*env)->ReleaseStringUTFChars(env, str, chars);
    }
}
*/
import "C"

import (
	"fmt"
	"log"

	"github.com/xjasonlyu/tun2socks/v2/engine"
)

//export Java_ru_everty_multilink_service_Tun2Socks_start
func Java_ru_everty_multilink_service_Tun2Socks_start(env *C.JNIEnv, clazz C.jclass, fd C.jint, proxy C.jstring, mtu C.jint) C.jint {
	cProxy := C.getStringChars(env, proxy)
	var proxyStr string
	if cProxy != nil {
		proxyStr = C.GoString(cProxy)
		C.releaseStringChars(env, proxy, cProxy)
	}
	if proxyStr == "" {
		proxyStr = "socks5://127.0.0.1:7890"
	}
	if mtu <= 0 {
		mtu = 1500
	}

	log.Printf("[Tun2Socks-JNI] Starting engine with fd=%d proxy=%s mtu=%d", int(fd), proxyStr, int(mtu))

	key := &engine.Key{
		Device:   fmt.Sprintf("fd://%d", int(fd)),
		Proxy:    proxyStr,
		MTU:      int(mtu),
		LogLevel: "info",
	}

	engine.Insert(key)
	engine.Start()
	log.Printf("[Tun2Socks-JNI] Engine started successfully")
	return 0
}

//export Java_ru_everty_multilink_service_Tun2Socks_stop
func Java_ru_everty_multilink_service_Tun2Socks_stop(env *C.JNIEnv, clazz C.jclass) C.jint {
	log.Printf("[Tun2Socks-JNI] Stopping engine")
	engine.Stop()
	return 0
}

func main() {}

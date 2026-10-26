# JNA finds native functions and structure fields by reflection, and the UniFFI binding is
# what it reflects on, so R8 must keep both whole (add-android-release D2).
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class uniffi.** { *; }
# JNA's desktop helpers name AWT classes Android does not have.
-dontwarn java.awt.**

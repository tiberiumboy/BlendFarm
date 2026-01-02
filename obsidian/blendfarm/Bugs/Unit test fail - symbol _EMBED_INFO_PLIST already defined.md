Currently unit test fails when generating a new context. I am not sure why I received this error message? I'm on a airplane with no wifi or internet connection whatsoever, so this makes troubleshooting a bit difficult to perform while in air. 

Expected behaviour - Should be able to run unit test and return result.

Actual behaviour - unable to run unit test as the compiler complains about symbol embed_info_plist is already defined.

**error****: symbol `_EMBED_INFO_PLIST` is already defined**

   **-->** src/routes/job.rs:301:23

    **|**

**301** **|**         let context = tauri::generate_context!("tauri.conf.json");

    **|**                       **^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^**

    **|**

    **=** **note**: this error originates in the macro `$crate::embed_info_plist_bytes` which comes from the expansion of the macro `tauri::generate_context` (in Nightly builds, run with -Z macro-backtrace for more info)


TODO: 
Try running with `-Z macro-backtrace` Chances are, need to clean and rebuild mac directory. Don't think I've ran into this problem again? Verify this.
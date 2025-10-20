Currently unit test fails when generating a new context. I am not sure why I receied this error message? I'm on a airplane with no wifi or internet connection whatsoever, so this makes troubleshooting a bit difficult to perform while in air. 

**error****: symbol `_EMBED_INFO_PLIST` is already defined**

   **-->** src/routes/job.rs:301:23

    **|**

**301** **|**         let context = tauri::generate_context!("tauri.conf.json");

    **|**                       **^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^**

    **|**

    **=** **note**: this error originates in the macro `$crate::embed_info_plist_bytes` which comes from the expansion of the macro `tauri::generate_context` (in Nightly builds, run with -Z macro-backtrace for more info)
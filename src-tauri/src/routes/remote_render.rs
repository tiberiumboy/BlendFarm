/* Dev blog:
- I really need to draw things out and make sense of the workflow for using this application.
I wonder why initially I thought of importing the files over and then selecting the files again to begin the render job?

For now - Let's go ahead and save the changes we have so far.
Next update - Remove Project list, and instead just allow user to create a new job.
when you create a new job, it immediately sends a new job to the server farm

for future features impl:
Get a preview window that show the user current job progress - this includes last frame render, node status, (and time duration?)
*/
use crate::AppState;
use blender::blender::Blender;
use build_html::{Html, HtmlElement, HtmlTag};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{command, AppHandle, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::Mutex;

/// List all of the available blender version.
#[command(async)]
pub async fn list_versions(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let manager = server.manager.read().await;
    let mut root = HtmlElement::new(HtmlTag::Div);
    let _ = manager.home.as_ref().iter().map(|b| {
        let version = match b.fetch_latest() {
            Ok(download_link) => download_link.get_version().clone(),
            Err(_) => Version::new(b.major, b.minor, 0),
        };
        let child = HtmlElement::new(HtmlTag::ListElement)
            .with_child(version.to_string().into())
            .into();
        root.add_child(child);
    });

    // let manager = server.manager.read().await;
    let _ = manager.get_blenders().iter().map(|b| {
        &root.add_child(
            HtmlElement::new(HtmlTag::ListElement)
                .with_child(b.get_version().to_string().into())
                .into(),
        );
    });

    Ok(root.to_html_string())

    // is this function working?
    // Ok(serde_json::to_string(&versions).expect("Unable to serialize version list!"))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlenderInfo {
    file_name: String,
    path: PathBuf,
    blend_version: Version,
    start: i32,
    end: i32,
    output: PathBuf,
    // could also provide other info like Eevee or Cycle?
}

#[command(async)]
pub async fn open_file_dialog(app: AppHandle) -> Result<String, String> {
    // tell tauri to open file dialog
    // with that file path we will run import_blend function.
    // else return nothing.
    let result = match app.dialog().file().blocking_pick_file() {
        Some(path) => import_blend(path.into_path().unwrap()).await.unwrap(),
        None => "".to_owned(),
    };
    Ok(result)
}

// change this to return HTML content of the info back.
#[command(async)]
pub async fn import_blend(path: PathBuf) -> Result<String, String> {
    // TODO: Is there any differences using file dialog from Javascript side or rust side?
    let file_name = match path.file_name() {
        Some(str) => str.to_str().unwrap().to_owned(),
        None => return Err("Should be a valid file!".to_owned()),
    };

    let data = match Blender::peek(&path).await {
        Ok(data) => data,
        Err(e) => return Err(e.to_string()),
    };

    let info = BlenderInfo {
        file_name,
        path,
        blend_version: data.last_version,
        start: data.frame_start,
        end: data.frame_end,
        output: data.output,
    };

    // instead of parsing the data into json - we need to format it into string type.
    let modal = HtmlElement::new(HtmlTag::Div)
        .with_child(
            HtmlElement::new(HtmlTag::ParagraphText)
                .with_child(info.file_name.into())
                .into(),
        )
        .to_html_string();

    Ok(modal)
    // html!(format!(
    // <div id="modal">
    //     <div class="modal-underlay"></div>
    //     <div class="modal-content">
    //         <form method="dialog">
    //             <h1>Create new Render Job</h1>
    //             <label>Project File Path:</label>
    //             <input type="text" defaultValue={info.path} placeholder="Project path" readOnly={true} />

    //             <br />
    //             <label>Blender Version:</label>
    //             <select defaultValue={props.info?.blend_version} onChange={(e) => setVersion(e.target.value)}>
    //                 {props.versions.map((item, index) => (
    //                 <option key={index} value={item}>{item}</option>
    //                 ))}
    //             </select>

    //             <div key="frameRangeEntry">
    //                 <label key="frameStartLabel" htmlFor="start">Start</label>
    //                 <input key="frameStartField"
    //                       name="start"
    //                       type="number"
    //                       defaultValue={props.info?.start}
    //                       onChange={(e) => setStartFrame(Number(e.target.value))}
    //                 />
    //                 <label key="frameEndLabel" htmlFor="end">End</label>
    //                 <input key="frameEndField"
    //                       name="end"
    //                       type="number"
    //                       defaultValue={props.info?.end}
    //                       onChange={(e) => setEndFrame(Number(e.target.value))}
    //                 />
    //             </div>

    //             <label>Output destination:</label>
    //             <input
    //                 type="text"
    //                 placeholder="Output Path"
    //                 id="output"
    //                 name="output"
    //                 defaultValue={props.info?.output}
    //                 readOnly={true}
    //                 onChange={(e) => setOutput(e.target.value)}
    //                 onClick={onDirectorySelect}
    //             />
    //             <menu>
    //                 <button type="button" value="cancel" onClick={() => props.onClose()}>Cancel</button>
    //                 <button type="submit">Ok</button>
    //             </menu>
    //         </form>
    //     </div>
    //   </div>
    // ))
    // let data = serde_json::to_string(&info).unwrap();
    // Ok(data)
}

#[command]
pub fn remote_render() -> String {
    HtmlElement::new(HtmlTag::Div)
        .with_child(
            HtmlElement::new(HtmlTag::ParagraphText)
                .with_child("Remote Settings".into())
                .into(),
        )
        .to_html_string()
}

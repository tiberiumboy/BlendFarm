import { BlenderProps } from "../props/blender_props";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import BlenderEntry from "../components/blender_entry";

export interface ServerSettingsProps {
  install_path: string,
  render_path: string,
  cache_path: string,
}

interface BlenderModalProps {
  showModal: boolean;
  versions: string[];
  onItemSelected(item: string): void;
  onDialogClose(): void;
}

function BlenderInstallerDialog(props: BlenderModalProps) {

  // not sure what this one is?
  const dialogRef = useRef<HTMLDialogElement>(null);

  // TODO: Find a better way to handle this. I cannot re-open the dialog again?
  // I can see that the showModal state has changed, but then won't change it again?
  // I think somewhere the state have detected a change, but then later realize I'm passing in the same value so no update was called?
  useEffect(() => {
    console.log("DialogRef:", dialogRef.current?.open);
    if (dialogRef.current?.open && !props.showModal) {
      dialogRef.current?.close()
    } else if (!dialogRef.current?.open && props.showModal) {
      dialogRef.current?.showModal()
    }
  }, [props]);

  return (
    <dialog ref={dialogRef} title="Install Blender Version from Web">
      {props.versions.map((v) => (
        <div className="item" onClick={() => props.onItemSelected(v)}>{v}</div>
      ))}
      <button onClick={props.onDialogClose}>Cancel</button>
    </dialog>
  )
}

export default function Setting(versions: string[]) {
  const [blenders, setBlenders] = useState(fetchBlenders);
  const [showModal, setShowModal] = useState(false);

  // TODO: Feels like I need to move these two states (setting+blendInstall) to App.tsx instead?
  const [setting, setSetting] = useState<ServerSettingsProps>({ install_path: '', render_path: '', cache_path: '' });

  useEffect(() => {
    fetchServerSettings();
  }, []);

  async function fetchServerSettings() {
    console.log("fetchserversettings");
    // is this possible? Does the JSON.parse handle this internally?
    let ctx: ServerSettingsProps | undefined = await invoke("get_server_settings");
    if (ctx === undefined) {
      return;
    }
    setSetting(ctx);
  }

  function fetchBlenders() {
    listBlenders(); // shouldn't need to wait.
    return [] as BlenderProps[];
  }

  async function listBlenders() {
    let ctx: any = await invoke("list_blender_installation");
    console.log("List Blender:", ctx);
    if (ctx == null) {
      return null;
    }

    const data: BlenderProps[] = JSON.parse(ctx);
    setBlenders(data);
  }

  async function setNewDirectoryPath(callback: (path: string) => void) {
    const filePath = await open({
      directory: true,
      multiple: false,
    });

    if (filePath != null) {
      // TODO: find a way to include the dash elsewhere
      callback(filePath + "/");
    }
  }

  // update the list collection to include the newly created blender object
  function onBlenderCreated(blender: BlenderProps): void {
    // to force the list to update, we cannot use shallow copy. React will only check the condition of the topmost component and refresh if the toplevel layer has changed.
    let list = [...blenders];
    list.push(blender);
    setBlenders(list);
  }

  function installBlenderFromVersion(version: String, blenderCreated: (blender: BlenderProps) => void) {
    // may need a safeguard here to alert user be patient while this program fetch, download, and install blender.
    invoke("fetch_blender_installation", { version }).then((ctx) => {
      if (ctx === null) {
        return;
      }

      blenderCreated(ctx as BlenderProps);
    })
  }

  function installBlenderFromLocal(blenderCreated: (blender: BlenderProps) => void) {
    open({
      multiple: false,
      filters: [
        {
          title: "Path to local blender installation",
          name: "Blender",
          extensions: ["exe", "zip", "dmg", "tar.xz", "app"], // how do I go about selecting app from linux? Linux app doesn't have extension AFAIK?
        },
      ],
    }).then((selected) => {
      if (selected != null) {
        invoke("add_blender_installation", { path: selected }).then((ctx: BlenderProps | any) => blenderCreated(ctx))
      }
    });
  }

  function handleItemSelected(item: string): void {
    handleDialogClosed();
    installBlenderFromVersion(item, onBlenderCreated);
  }

  function handleDialogClosed(): void {
    setShowModal(false);
  }

  return (
    <div className="content">
      <h1>Settings</h1>
      <p>
        Here we list out all possible configuration this tool can offer to user.
        Exposing rich and deep component to fit your production flow
      </p>
      <h3>Local Settings</h3>
      <div className="group">
        <form>
          <label style={{ float: "left" }}>
            Blender Installation Path:
          </label>
          <input
            className="form-input"
            type="text"
            placeholder="Blender Installation Path"
            value={setting.install_path}
            readOnly={true}
            onClick={async () => setNewDirectoryPath((path) => setting.install_path = path)}
          />

          <br />

          <label>
            Blender File Cache Path:
          </label>

          <input
            type="text"
            placeholder="Path to blender file working directory"
            name="blend_dir"
            className="form-input"
            readOnly={true}
            value={setting.cache_path}
            onClick={async () => setNewDirectoryPath((path) => setting.cache_path = path)}
          />

          <br />

          <label>
            Render cache directory:
          </label>
          <input
            className="form-input"
            type="text"
            placeholder="Path to completed render frames for cache"
            name="render_path"
            value={setting.render_path}
            readOnly={true}
            onClick={async () => setNewDirectoryPath((path) => setting.render_path = path)}
          />

        </form>
      </div>
      <h3>
        Blender Installation
      </h3>
      <button onClick={() => installBlenderFromLocal(onBlenderCreated)}>
        Add from Local Storage
      </button>
      <button onClick={() => setShowModal(true)}>
        Install version
      </button>
      <div className="group">
        {blenders.map((blender: BlenderProps) => (
          (blender.onDelete = listBlenders),
          BlenderEntry(blender)
        ))}
      </div>

      <BlenderInstallerDialog
        showModal={showModal}
        versions={versions}
        onItemSelected={handleItemSelected}
        onDialogClose={handleDialogClosed} />
      {/* Todo Display the list of blender installation stored in serversettings config */}
    </div >
  );
}

import { BlenderProps } from "../props/blender_props";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import BlenderEntry from "../components/blender_entry";

export interface ServerSettingsProps {
  render_dir: string;
  blend_dir: string;
}

interface BlenderModalProps {
  showModal: boolean;
  versions: string[];
  onItemSelected(item: string): void;
}

function BlenderInstallerDialog(props: BlenderModalProps) {

  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    if (dialogRef.current?.open && !props.showModal) {
      dialogRef.current?.close()
    } else if (!dialogRef.current?.open && props.showModal) {
      dialogRef.current?.showModal()
    }
  }, [[props.showModal]]);

  return (
    <dialog ref={dialogRef} title="Install Blender Version from Web">
      {props.versions.map((v) => (
        <div className="item" onClick={() => props.onItemSelected(v)}>{v}</div>
      ))}
      <button onClick={() => dialogRef.current?.close()}>Cancel</button>
    </dialog>
  )
}

export default function Setting(versions: string[]) {
  const [blenders, setBlenders] = useState(fetchBlenders);
  const [showModal, setShowModal] = useState(false);

  // TODO: Feels like I need to move these two states (setting+blendInstall) to App.tsx instead?
  const [blendInstall, setBlendInstall] = useState("/");
  const [setting, setSetting] = useState({ render_dir: '', blend_dir: '' } as ServerSettingsProps);

  useEffect(() => {
    fetchServerSettings();
  }, []);

  function fetchServerSettings() {
    console.log("fetchserversettings");
    invoke("get_server_settings").then((data: ServerSettingsProps | any) => setSetting(data));
  }

  function fetchBlenders() {
    listBlenders();
    return [] as BlenderProps[];
  }

  function listBlenders() {
    invoke("list_blender_installation").then((ctx) =>
      setBlenders(JSON.parse(ctx + "")),
    );
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
    let list = blenders;
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
          extensions: ["exe", "zip", "dmg", "tar.xz"], // how do I go about selecting app from linux? Linux app doesn't have extension AFAIK?
        },
      ],
    }).then((selected) => {
      if (selected != null) {
        invoke("add_blender_installation", { path: selected }).then((ctx: BlenderProps | any) => blenderCreated(ctx))
      }
    });
  }

  function handleItemSelected(item: string): void {
    setShowModal(false);
    installBlenderFromVersion(item, onBlenderCreated);
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
          <span style={{ display: "block", overflow: "hidden", }}>
            <input
              style={{ width: '100%' }}
              type="text"
              placeholder="Blender Installation Path"
              value={blendInstall}
              readOnly={true}
              onClick={async () => setNewDirectoryPath(setBlendInstall)}
            />
          </span>

          <br />

          <label>
            Blender File Cache Path:
          </label>

          <input
            type="text"
            placeholder="Path to blender file working directory"
            name="blend_dir"
            readOnly={true}
            value={setting.blend_dir}
            onClick={async () => setNewDirectoryPath((path) => setting.blend_dir = path)}
          />

          <br />

          <label>
            Render cache directory:
          </label>

          <input
            type="text"
            placeholder="Path to completed render frames for cache"
            name="render_dir"
            value={setting.render_dir}
            readOnly={true}
            onClick={async () => setNewDirectoryPath((path) => setting.render_dir = path)}
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

      <BlenderInstallerDialog showModal={showModal} versions={versions} onItemSelected={handleItemSelected} />
      {/* Todo Display the list of blender installation stored in serversettings config */}
    </div >
  );
}

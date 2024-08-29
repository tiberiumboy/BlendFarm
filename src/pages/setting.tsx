import { BlenderProps } from "../props/blender_props";
import { invoke } from "@tauri-apps/api/core";
import { ReactElement, useEffect, useState } from "react";
import { open } from "@tauri-apps/api/dialog";
import { CiCirclePlus } from "react-icons/ci";
import BlenderEntry from "../components/blender_entry";

export class ServerSettingsProps {
  public render_dir: string;
  public blend_dir: string;

  constructor(render_dir: string, blend_dir: string) {
    this.render_dir = render_dir;
    this.blend_dir = blend_dir;
  }
}

export default function Setting() {
  const [blenders, setBlenders] = useState(fetchBlenders);
  const [blendInstall, setBlendInstall] = useState("/");
  const [blendFiles, setBlendFiles] = useState("");
  const [setting, setSetting] = useState({} as ServerSettingsProps);

  useEffect(() => {
    fetchServerSettings();
  }, []);


  function fetchServerSettings() {
    invoke("get_server_settings").then((ctx) =>
      setSetting(JSON.parse(ctx + "")),
    );
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

  return (
    <div className="content">
      <h1>Settings</h1>
      <p>
        Here we list out all possible configuration this tool can offer to user.
        Exposing rich and deep component to fit your production flow
      </p>
      <h2>Local Settings</h2>
      <div className="group">
        <form>
          <label>
            Blender Installation Path:
          </label>
          <input
            type="text"
            placeholder="Blender Installation Path"
            id="blender_dir"
            name="blender_dir"
            value={blendInstall}
            readOnly={true}
            onClick={async () => setNewDirectoryPath(setBlendInstall)}
          />

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
      <h2>
        Blender Installation
        <CiCirclePlus
          onClick={(e: any) => {
            e.preventDefault();
            open({
              multiple: false,
              // filters: [
              //   {
              //     name: "Blender",
              //     // extensions: ["exe", "dmg", ""], // how do I go about selecting app from linux? Linux app doesn't have extension AFAIK?
              //   },
              // ],
            }).then((selected) => {
              if (selected != null) {
                invoke("add_blender_installation", { path: selected }).then(
                  listBlenders,
                );
              }
            });
          }}
        />
      </h2>
      <div className="group">
        {blenders.map((blender: BlenderProps) => (
          (blender.onDelete = listBlenders),
          BlenderEntry(blender)
        ))}
      </div>
      {/* Todo Display the list of blender installation stored in serversettings config */}
    </div >
  );
}

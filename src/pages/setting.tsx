import { BlenderProps } from "../props/blender_props";
import { invoke } from "@tauri-apps/api/tauri";
import { useState } from "react";
import { open } from "@tauri-apps/api/dialog";
import { CiCirclePlus } from "react-icons/ci";
import BlenderEntry from "../components/blender_entry";

export default function Setting() {
  const [blenders, setBlenders] = useState(fetchBlenders);

  function fetchBlenders() {
    listBlenders();
    return [] as BlenderProps[];
  }

  function listBlenders() {
    invoke("list_blender_installation").then((ctx) =>
      setBlenders(JSON.parse(ctx + "")),
    );
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
        {/* need to find a way to expose server configs to let the user personalize their node/server settings */}
        {/* TODO: Find out how to get folder path and struct like in the other dialog form */}
        <form>
          Blender Installation Path:
          <input
            type="text"
            placeholder="Blender Installation Path"
            id="blender_dir"
            name="blender_dir"
            value={"/Users/Shared/"}
            readOnly={true}
            onClick={async (e: any) => {
              const filePath = await open({
                directory: true,
                multiple: false,
              });
              if (filePath != null) {
                // TODO: find a way to include the dash elsewhere
                e.target.value = filePath + "/";  // this is quite annoying...
              }
            }}
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
    </div>
  );
}

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
      <h2>
        Here we list out all possible configuration this tool can offer to user.
        Exposing rich and deep component to fit your production flow
      </h2>
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
      <div className="group">
        {blenders.map((blender: BlenderProps) => (
          <div>
            {/* TODO: Find a way to only extract the file name here? */}
            {BlenderEntry(blender)}
          </div>
        ))}
      </div>
      {/* Todo Display the list of blender installation stored in serversettings config */}
    </div>
  );
}

import { invoke } from "@tauri-apps/api/core";
import { RenderNodeProps, ComputerSpec } from "./render_node";
import { useState } from "react";
import RenderNode from "./render_node";
import { listen } from "@tauri-apps/api/event";

export interface node {

}

export default function NodeWindow() {
  const [nodes, setNodes] = useState<RenderNodeProps[]>([]);

  const unlisten_status = listen('node_status', (event: any) => {
    let id = event.payload[0];  // which node is reporting the status message
    let msg = event.payload[1]; // the content of the message
  })

  // This is later fetch after the node sends the host information about the specs.
  const unlisten_identity = listen('node_discover', (event: any) => {
    console.log("Node discovered");
    // 0 is peer_id in base58, 1 is computer specs object
    let id = event.payload[0];
    // 1 is the computer spec payload
    let spec: ComputerSpec = event.payload[1];

    let node: RenderNodeProps = { name: id, spec, status: "Idle" };
    let tmp = [...nodes];
    tmp.push(node); 
    setNodes(tmp);
  })

  // this probably won't happen... 
  const unlisten_disconnect = listen('node_disconnect', (event: any) => {
    console.log("Node Disconnected");
    let tmp = [...nodes];
    let id = event.payload;
    tmp.filter((t) => t.name == id);
    console.log("Node disconnected", id, tmp);
    setNodes(tmp);
  });

  // TODO: Find a way to make this node selectable, and refresh the screen to display node property and information (E.g. Blender preview window, Activity monitor, specs, files completed, etc.)
  function nodeWindow() {
    return (
      <div>
        {/* Show the activity of the computer progress */}
        <h2>Computer Nodes</h2>
        <div className="group" id="RenderNodes">
          {nodes.map((node) =>
            <div>{RenderNode(node)}</div> 
          )}
        </div>
      </div>
    );
  }

  return (
    <div>
      {nodeWindow()}
    </div>
  );
}

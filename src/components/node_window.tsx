import { invoke } from "@tauri-apps/api/core";
import { RenderNodeProp, ComputerSpec } from "./render_node";
import { useEffect, useState } from "react";
import RenderNode from "./render_node";

export interface node {

}

export default function NodeWindow() {
  // connect to surreal db from here
  const [nodes, setNodes] = useState<RenderNodeProp[]>([]);

  useEffect(() => {
    getWorkers();
  }, []);

  // Ok so we're fetching the list of workers here?
  function getWorkers() {
    invoke("list_workers").then((ctx: any) => {
      console.log(ctx);
      const workers = JSON.parse(ctx);
      setNodes(workers);
    })
  }
  

  // TODO: Find a way to make this node selectable, and refresh the screen to display node property and information (E.g. Blender preview window, Activity monitor, specs, files completed, etc.)
  function nodeWindow() {
    return (
      <div>
        {/* Show the activity of the computer progress */}
        <h2 onClick={getWorkers}>Computer Nodes</h2>
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

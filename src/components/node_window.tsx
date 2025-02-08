import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { RenderNodeProp, ComputerSpec } from "./render_node";
import { useEffect, useState } from "react";
import RenderNode from "./render_node";

// not sure what this is?
export interface node {

}

export default function NodeWindow() {
  // connect to surreal db from here
  const [nodes, setNodes] = useState<RenderNodeProp[]>([]);

  useEffect(() => {
    getWorkers();
    listen('node', () => getWorkers());
  }, []);

  // Ok so we're fetching the list of workers here?
  function getWorkers() {
    invoke("list_workers").then((ctx: any) => {
      const workers = JSON.parse(ctx);
      setNodes(workers);
    })
  }

  // TODO: Find a way to make this node selectable, and refresh the screen to display node property and information (E.g. Blender preview window, Activity monitor, specs, files completed, etc.)
  function nodeWindow() {
    return (
      <div>
        {/* Show the activity of the computer progress */}
        <h2 hx-on={getWorkers}>Computer Nodes</h2>
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

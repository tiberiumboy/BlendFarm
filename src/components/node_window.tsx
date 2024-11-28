import { invoke } from "@tauri-apps/api/core";
import { RenderNodeProps } from "./render_node";
import { useState } from "react";
import RenderNode from "./render_node";
import { listen } from "@tauri-apps/api/event";

export default function NodeWindow() {
  // Why is fetchNodes not being called?
  const [nodes, setNodes] = useState<String[]>([]);

  // TODO: Find a way to access destructor to properly unsubscribe global events.

  function fetchNodes() {
    const initialNodes: RenderNodeProps[] = [];
    return initialNodes;
  }

  //TODO: read more into this https://v2.tauri.app/develop/calling-frontend/
  // ok this works. Just need to find a way to subscribe on component start, and then unlisten when deconstruct.
  // Problem - I'm getting permission issue?`
  // listen('node_joined', (event) => {
  //   console.log(event);
  // });

  listen<string>('node_discover', (event: any) => {
    console.log("Node connected", event);
    let tmp = nodes;
    tmp.push(event);
    setNodes(tmp);
  });

  listen('node_disconnect', (event: any) => {
    let tmp = nodes;
    let result = tmp.filter((t) => t == event);
    setNodes(result);
  });

  function nodeWindow() {
    return (
      <div>
        {/* Show the activity of the computer progress */}
        <h2>Computer Nodes</h2>
        <div className="group" id="RenderNodes">
          {/* {nodes.map((node: RenderNodeProps) => RenderNode(node))}
           */}
          {nodes.map((node: String) => <h3>node</h3>)}
        </div>
      </div>
    );
  }

  return (
    <div>
      {/* I'm concern about dialog window size dimension */}
      {nodeWindow()}
    </div>
  );
}

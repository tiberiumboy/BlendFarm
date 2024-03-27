import { invoke } from "@tauri-apps/api";

export type RenderNodeProps = {
  id: string;
  name?: string;
  onDataChanged?: () => void;
};

export default function RenderNode(node: RenderNodeProps) {
  function deleteNode() {
    let params = {
      id: node.id,
    };
    invoke("delete_node", params).then(node.onDataChanged); // then we should signal a refresh somehow?
  }

  return (
    <div key={node.id}>
      <h5>{node.name}</h5>
      <button onClick={deleteNode}>Remove</button>
    </div>
  );
}

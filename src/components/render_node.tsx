import { invoke } from "@tauri-apps/api";
import * as ciIcon from "react-icons/ci";

export interface RenderNodeProps {
  id: string;
  name?: string;
  onDataChanged?: () => void;
}

export default function RenderNode(node: RenderNodeProps) {
  function deleteNode() {
    let params = {
      id: node.id,
    };
    invoke("delete_node", params).then(node.onDataChanged); // then we should signal a refresh somehow?
  }

  function moreOption() {
    let add_modal = document.getElementById("add_modal");
    add_modal.showModal();
  }

  function closeOption() {
    let add_modal = document.getElementById("add_modal");
    add_modal.close();
  }

  return (
    <div>
      <table>
        <tr>
          <td style={{ width: "100%" }}>{node.name}</td>
          <td>
            <ciIcon.CiTrash onClick={deleteNode} />
          </td>
          <td>
            <ciIcon.CiCircleMore onClick={moreOption} />
          </td>
        </tr>
      </table>
      <dialog id="add_modal">
        <button onClick={deleteNode}>Delete</button>
        <button onClick={closeOption}>Cancel</button>
      </dialog>
    </div>
  );
}

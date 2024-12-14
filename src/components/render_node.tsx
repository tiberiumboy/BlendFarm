// import { invoke } from "@tauri-apps/api/core";
// import { CiTrash, CiCircleMore } from "react-icons/ci";

export interface RenderNodeProp {
  id: string;
  spec?: ComputerSpec;
  status: string;
}

export type ComputerSpec = {
  host: string;
  cpu: string;
  gpu?: string;
  arch: string;
  os: string;
  cores: number;
  memory: number;
}

// it would be nice to be able to make this "targeted" update from the backend services.
// I want to be able to send status message + heartbeat signal to target rendernodeprops.

export default function RenderNode(node: RenderNodeProp) {
  return (
    <div key={"Node_" + node.id}>
      <table>
        <tbody>
          <tr>
            <td style={{ width: "100%" }}>
              <div>{node.spec?.host ?? "Loading"}</div>
              <div>{node.spec?.os ?? ""} | {node.spec?.arch ?? ""}</div>
              <div>{node.status}</div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}

import { CiTrash } from "react-icons/ci";
import { MdEdit, MdOutlineMovie } from "react-icons/md";
import { invoke } from "@tauri-apps/api/tauri";

export interface ProjectFileProps {
  id: string;
  file_name: string;
  src: string;
  onDataChanged?: () => void;
}

// todo: expose function controls here. props event handler?
export default function (props: ProjectFileProps) {
  const getFileName = () => {
    return props.src?.split("/").pop()?.split("\\").pop();
  };

  const deleteProject = () =>
    invoke("delete_project", { id: props.id }).then(props.onDataChanged);

  // this should really open a new dialog and asking for which machine to use for this job...
  // but ok
  const createNewJob = () =>
    invoke("create_job", { id: props.id }).then(props.onDataChanged);

  return (
    <div className="item" key={props.id} id={props.id}>
      <table>
        <tr>
          <td style={{ width: "100%" }}>
            <p>{getFileName()}</p>
            <p>{props.src}</p>
          </td>

          {/* <td>
            <MdEdit onClick={editProject} />
          </td> */}
          <td>
            <MdOutlineMovie onClick={createNewJob} />
          </td>
          <td>
            <CiTrash onClick={deleteProject} />
          </td>
        </tr>
      </table>
    </div>
  );
}

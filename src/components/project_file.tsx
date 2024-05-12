import { CiTrash } from "react-icons/ci";
import { MdOutlineMovie } from "react-icons/md";
import { invoke } from "@tauri-apps/api/tauri";

export interface ProjectFileProps {
  id: string;
  file_name: string;
  src: string;
  onDataChanged?: () => void;
  onRequestNewJob: (project: ProjectFileProps) => void;
}

// todo: expose function controls here. props event handler?
export default function (props: ProjectFileProps) {
  const getFileName = () => {
    return props.src?.split("/").pop()?.split("\\").pop();
  };

  const deleteProject = () =>
    invoke("delete_project", { projectFile: props }).then(props.onDataChanged);

  // this should really open a new dialog and asking for which machine to use for this job...
  // but ok
  const createNewJob = () => props.onRequestNewJob(props);

  return (
    <div className="item" key={props.id} id={props.id}>
      <table>
        <tbody>
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
        </tbody>
      </table>
    </div>
  );
}

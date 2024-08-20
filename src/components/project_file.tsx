import { CiTrash } from "react-icons/ci";
import { MdOutlineMovie } from "react-icons/md";
import { invoke } from "@tauri-apps/api/tauri";

export interface ProjectFileProps {
  // do I need this?
  id: string;
  // extract file name from src instead.
  src: string;
  onDataChanged?: () => void;
  onRequestNewJob?: (project: ProjectFileProps) => void;

}

// todo: expose function controls here. props event handler?
export default function ProjectFile(props: ProjectFileProps) {
  const getFileName = () => {
    return props.src?.split("/").pop()?.split("\\").pop();
  };

  const deleteProject = () =>
    invoke("delete_project", { projectFile: props }).then(props.onDataChanged);

  const createNewJob = () => props.onRequestNewJob ? props.onRequestNewJob(props) : null;

  return (
    <div className="item" key={props.id}>
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

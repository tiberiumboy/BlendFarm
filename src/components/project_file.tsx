import { CiTrash } from "react-icons/ci";
import { MdOutlineMovie } from "react-icons/md";
import { invoke } from "@tauri-apps/api/core";

export interface ProjectFileProps {
  file_name: String;
  path: String;
  onDataChanged?: () => void;
  onRequestNewJob?: (project: ProjectFileProps) => void;
}

// todo: expose function controls here. props event handler?
export default function ProjectFile(props: ProjectFileProps) {

  const deleteProject = () =>
    invoke("delete_project", { projectFile: props }).then(props.onDataChanged);

  const createNewJob = () => props.onRequestNewJob ? props.onRequestNewJob(props) : null;

  return (
    <div className="item">
      <table>
        <tbody>
          <tr>
            <td style={{ width: "100%" }}>
              <p>{props.file_name}</p>
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

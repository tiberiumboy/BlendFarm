import { CiTrash, CiCircleMore } from "react-icons/ci"
import { invoke } from "@tauri-apps/api/tauri";
import { BlenderProps } from "../props/blender_props";

export default function BlenderEntry(props: BlenderProps) {

    function handleDelete() {
        invoke("remove_blender_installation", { blender: props }).then(props.onDelete);
    }

    return (
        // Todo Find a way to make the full path visible when the mouse hover over this?
        <div className="item" key={props.version + "_" + props.executable}>
            <table>
                <tbody>
                    <tr>
                        <td style={{ width: "100%" }}>
                            Blender {props.version}
                        </td>
                        <td>
                            <CiCircleMore />
                        </td>
                        <td>
                            <CiTrash onClick={handleDelete} />
                        </td>
                    </tr>
                </tbody>
            </table>
            {/* Find a way to get the blender's icon? Do we need it? */}
            {/* <div>{getFileName()}</div> */}
        </div>
    )
}
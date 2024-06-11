import { CiTrash, CiCircleMore } from "react-icons/ci"

export interface BlenderEntryProps {
    executable: String,
    version: String,
    onDelete?: () => void,
};

export default function BlenderEntry(props: BlenderEntryProps) {

    function getFileName() {
        const list = props.executable.split("/");
        console.log(list);
        return props.executable
    }

    return (
        // Todo Find a way to make the full path visible when the mouse hover over this?
        <div className="item" key={props.version + "_" + props.executable}>
            <table>
                <tbody>
                    <tr>
                        <td style={{ width: "100%" }}>
                            <p>Blender {props.version}</p>
                        </td>
                        <td>
                            <CiCircleMore />
                        </td>
                        <td>
                            <CiTrash onClick={props.onDelete} />
                        </td>
                    </tr>
                </tbody>
            </table>
            {/* Find a way to get the blender's icon? Do we need it? */}
            {/* <div>{getFileName()}</div> */}
        </div>
    )
}
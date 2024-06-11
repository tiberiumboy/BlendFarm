export interface BlenderEntryProps {
    executable: String,
    version: String,
    onDelete?: () => void,
};

export default function BlenderEntry(props: BlenderEntryProps) {


    return (
        <div>
            <div>{props.executable}</div>
            <div>{props.version}</div>
            <button>Modify</button>
            <button onClick={props.onDelete}>Delete</button>
        </div>
    )
}
import { RenderNodeProp } from "../components/render_node";

/*
    This page will display all of the computer information related to the worker that's running blendfarm.
    In essence - Display Worker's current progress, specs, and computer monitor
*/
export default function Workers(node: RenderNodeProp) {
    return (
        <div>
            <h1>Computer: {node.spec?.host}</h1>
            <h3>Specs</h3>
            <p>CPU: {node.spec?.cpu}</p>
            <p>Ram: { (node.spec?.memory ?? 0 ) / ( 1024 * 1024 )} Gb</p>
            <p>OS: {node.spec?.os} | {node.spec?.arch}</p>
            {/* how can I make a if condition to display GPU if it's available? */}
            <p>GPU: {node.spec?.gpu}</p>
            
            <h3>Current Task:</h3>
            <p>Task: None</p>
            <p>Frame: 0/0</p>

            <h3>Monitor</h3>
            {/* Draw a Linegraph to display CPU/Mem usage */}
        </div>
    )
}
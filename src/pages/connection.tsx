import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useState } from "react";

async function Connection() {
    const [ip, setIp] = useState("");


    async function handleSubmit(e: any) {
        e.preventDefault();
        let result = await invoke("create_node");
        console.log(result);
        return false;
    }

    const unlisten = await listen('list_node', (event) => {
        console.log(event);
    })

    return (
        <div className="content">
            <h3>Connection</h3>
            <form onSubmit={handleSubmit}>
                <label>Internet Protocol Address</label>
                <input type="text" placeholder="IP Address" id="ip" name="ip" />
                <br></br>
                <input type="number" placeholder="Port" id="port" name="port" value={15000} />
                <button type="submit">Connect</button>
            </form>
            <br></br>
            <h4>Here we will show all of the render client we have previously connected to</h4>
            <h5>We can also add new render node to this entry.</h5>
        </div >
    );
}

export default Connection;
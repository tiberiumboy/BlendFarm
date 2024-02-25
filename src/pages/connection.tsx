import { useState } from "react";

function Connection() {
    const [ip, setIp] = useState("");

    async function handleSubmit(e: any) {
        e.preventDefault();

        return false;
    }

    return (
        <div className="content">
            <h3>Connection</h3>
            <form onSubmit={handleSubmit}>
                <label>Internet Protocol Address</label>
                <input type="text" id="ip" name="ip" />
                <br></br>
                <label>Port</label>
                <input type="number" id="port" name="port" />
                <button type="submit">Connect</button>
            </form>
            <br></br>
            <h4>Here we will show all of the render client we have previously connected to</h4>
            <h5>We can also add new render node to this entry.</h5>
        </div >
    );
}

export default Connection;
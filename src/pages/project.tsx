import { invoke } from "@tauri-apps/api/tauri";

function Project() {
    // here we will hold the application context and inforamtion to make modification
    // this is where we will store our data state
    // and information across the tools we expose.

    async function addtoProjectList() {
        // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
        await invoke("add_project");
    }

    // TODO: replace any to strongly typed value
    async function editProject(id: any) {
        // todo - find a way to pass argument here and what kind of details do we need? Can we parse an object?
        await invoke("edit_project", id);
    }

    // Todo find a way to load previous project settings here!
    async function loadProjectList() {
        let message = await invoke("load_project_list");
        console.log(message);
    }

    loadProjectList();


    return (
        <div className="content">
            <h3>Load Blender</h3>
            <button id="load_project" type="submit" onClick={addtoProjectList}>
                Load Blend file
            </button>

            {/* Show the list of project available here */}
            <div className="group" id="project-list">
                <table>
                    <tr>
                        <th>Node Name</th>
                        <th>Status</th>
                        <th>Progress</th>
                        <th>Action</th>
                    </tr>
                    <tr>
                        <td>Localhost</td>
                        <td>Idle</td>
                        {/* <!-- progress bar --> */}
                        <td>

                        </td>
                        {/* <!-- context menu --> */}
                        <td></td>
                    </tr>
                </table>
            </div>


        </div>
    );
}

export default Project;
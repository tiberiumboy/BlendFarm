import "./App.css";
import "./styles.css";
import Sidebar from "./components/side_bar";
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Setting from "./pages/setting";
import RemoteRender from "./pages/remote_render";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RenderJobProps } from "./components/render_job";
import { listen } from "@tauri-apps/api/event";

function App() {
  const [versions, setVersions] = useState([] as string[]);
  const [jobs, setJobs] = useState(fetchJobs);

  listen("tauri://file-drop", event => { console.log(event); });

  const unlisten_job_complete = listen("job_image_complete", (event: any) => {
    console.log(event); // should be a path which we can load into the job props?
    let id = event.payload[0];
    // let frame = event.payload[1];
    let path = event.payload[2];
    let tmp = [...jobs];
    // I would have expect that this should not fail.. but if it does, I need to do something about it.
    let index = tmp.findIndex(j => j.id == id);
    console.log(tmp, index);
    if (index === -1) {
      console.error("Unable to find matching id from local collection to backend id? What did you do?");
      return;
    }

    if (tmp[index].renders === undefined) {
      tmp[index].renders = [path];
    } else {
      tmp[index].renders.unshift(path);
    }
    setJobs(tmp);
  })

  function loadJobs() {
    invoke("list_jobs").then((ctx: any) => {
      if (ctx == null) {
        return;
      }
      const data: RenderJobProps[] = JSON.parse(ctx);
      setJobs(data);
    });
  }

  function fetchJobs(): RenderJobProps[] {
    loadJobs();
    return [];
  }

  useEffect(() => {
    listVersions();
  }, []);

  // how can I go about getting the list of blender version here?
  function listVersions() {
    // this function is expensive! Consider refactoring this so that it's not costly!
    invoke("list_versions").then((ctx: any) => {
      const data: string[] = JSON.parse(ctx);
      data.sort();
      data.reverse();
      setVersions(data);
    });
  }

  function onJobCreated(job: RenderJobProps): void {
    const data = [...jobs];
    data.push(job);
    console.log("OnJobCreated", data);
    setJobs(data);
  }

  return (
    <div>
      <Router>
        <Sidebar />
        <Routes>
          <Route path='/' Component={() => RemoteRender({ versions: versions, jobs: jobs, onJobCreated: onJobCreated })} />
          <Route path='/remote_render' Component={() => RemoteRender({ versions, jobs, onJobCreated })} />
          <Route path='/setting' Component={() => Setting(versions)} />
        </Routes>
      </Router>
    </div>
  );
}

export default App;

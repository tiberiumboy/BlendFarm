// import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";
import "./styles.css";
import Sidebar from "./components/side_bar";
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Setting from "./pages/setting";
// import LiveView from "./pages/live_view";
import RemoteRender from "./pages/remote_render";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RenderJobProps } from "./components/render_job";

function App() {
  const [versions, setVersions] = useState([] as string[]);
  const [jobs, setJobs] = useState(fetchJobs);

  // TODO: Find a way to load current jobs collection in the server settings?
  function loadJobs() {
    // wouldn't this create a loop feedback?
    invoke("list_jobs").then((ctx: any) => {
      // this spammed out of control...
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
          {/* TODO: This is a experimental feature - ignore for this right now as this requires remote_render working first!
          < Route path='/liveview' Component={LiveView} /> 
           */}
        </Routes>
      </Router>
    </div>
  );
}

export default App;

import React from "react";
import Tabs from "@material-ui/core/Tabs";
import Tab from "@material-ui/core/Tab";
import FailedContent from "./failedContent";
import QueueContent from "./queueContent";
import Workers from "./workers";
import { useHistory, Switch, Route, useLocation } from "react-router-dom";

interface Props {
  queues: Array<string>;
}

const Queues: React.FC<Props> = ({ queues }) => {
  let history = useHistory();
  const location = useLocation();
  function renderQueues() {
    return queues.map(queue => (
      <Route key={queue} path={`/${queue}`}>
        <QueueContent key={queue} queue={queue} />
      </Route>
    ));
  }

  function currentTab(): number {
    const idx = [...queues, "failed", "workers"].indexOf(
      location.pathname.replace(/^\//, "")
    );
    return idx === -1 ? 0 : idx;
  }
  return (
    <div>
      <Tabs value={currentTab()}>
        {queues.map(q => (
          <Tab key={q} label={q} onClick={() => history.push(`/${q}`)} />
        ))}
        <Tab
          key="failed"
          label="Failed"
          onClick={() => history.push("/failed")}
        />
        <Tab
          key="workers"
          label="Workers"
          onClick={() => history.push("/workers")}
        />
      </Tabs>
      <Switch>
        {renderQueues()}
        <Route path="/failed">
          <FailedContent key="failed" />
        </Route>
        <Route path="/workers">
          <Workers key="workers" />
        </Route>
      </Switch>
    </div>
  );
};

export default Queues;

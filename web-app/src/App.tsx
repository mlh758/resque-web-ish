import React, { useState, useEffect } from "react";
import Bar from "./components/bar";
import Queue from "./components/queues";
import LinearProgress from "@material-ui/core/LinearProgress";
import Typography from "@material-ui/core/Typography";
import { useHistory, useLocation } from "react-router-dom";
import BASE_PATH from "./utils/basePath";

interface ResqueStats {
  successCount: Number;
  failureCount: Number;
  queues: Array<string>;
}

const App: React.FC = () => {
  const [stats, setStats] = useState<ResqueStats>({
    successCount: 0,
    failureCount: 0,
    queues: []
  });
  const [loading, setLoading] = useState(true);
  const [failed, setFailed] = useState(false);
  let history = useHistory();
  const location = useLocation();
  useEffect(() => {
    if (loading) return;
    if (!stats.queues.length) {
      history.push("/failed");
      return;
    }
    if (stats.queues.length > 0 && location.pathname === "/") {
      history.push(`/${stats.queues[0]}`);
    }
  }, [stats.queues, history, loading]);
  useEffect(() => {
    fetch(`${BASE_PATH}/api/stats`)
      .then(response => {
        if (response.ok) {
          return response.json();
        }
        throw new Error("unable to load stats");
      })
      .then(data => {
        setStats({
          successCount: data.success_count,
          failureCount: data.failure_count,
          queues: data.available_queues
        });
      })
      .catch(() => {
        setFailed(true);
      })
      .finally(() => setLoading(false));
  }, []);
  const renderQueues = () => {
    if (loading) {
      return <LinearProgress variant="query" />;
    }
    return <Queue queues={stats.queues} />;
  };
  return (
    <div className="App">
      <Bar
        successCount={stats.successCount}
        failureCount={stats.failureCount}
      />
      {!failed && renderQueues()}
      {failed && (
        <Typography variant="subtitle1">
          Unable to load Resque information
        </Typography>
      )}
    </div>
  );
};

export default App;

import React, { useState, useEffect } from 'react';
import Bar from './components/bar';
import Queue from './components/queues';
import LinearProgress from '@material-ui/core/LinearProgress';
import Typography from '@material-ui/core/Typography';

interface ResqueStats {
  successCount: Number,
  failureCount: Number,
  queues: Array<string>,
}

const App: React.FC = () => {
  const [stats, setStats] = useState<ResqueStats>({ successCount: 0, failureCount: 0, queues: [] });
  const [loading, setLoading] = useState(true);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    fetch('stats')
      .then((response) => {
        if (response.ok) {
          return response.json();
        }
        throw new Error('unable to load stats')
      })
      .then((data) => {
        setStats({
          successCount: data.success_count,
          failureCount: data.failure_count,
          queues: data.available_queues,
        })
      }).catch(() => {
        setFailed(true);
      }).finally(() => setLoading(false));
  }, []);
  const renderQueues = () => {
    if (loading) {
      return (<LinearProgress variant="query" />)
    }
    return (<Queue queues={stats.queues} />);
  }
  return (
    <div className="App">
      <Bar successCount={stats.successCount} failureCount={stats.failureCount} />
      {!failed && renderQueues()}
      {failed && <Typography variant="subtitle1">Unable to load Resque information</Typography>}
    </div>
  );
}

export default App;

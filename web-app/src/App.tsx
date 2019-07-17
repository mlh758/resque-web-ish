import React, { useState, useEffect } from 'react';
import Bar from './components/bar';
import Queue from './components/queues';
import LinearProgress from '@material-ui/core/LinearProgress';

interface ResqueStats {
  successCount: Number,
  failureCount: Number,
  queues: Array<string>,
}

const App: React.FC = () => {
  const [stats, setStats] = useState<ResqueStats>({ successCount: 0, failureCount: 0, queues: [] });
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    fetch('stats')
      .then((response) => response.json())
      .then((data) => {
        setStats({
          successCount: data.success_count,
          failureCount: data.failure_count,
          queues: data.available_queues,
        });
        setLoading(false);
      });
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
      {renderQueues()}
    </div>
  );
}

export default App;

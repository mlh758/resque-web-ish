import React, { useState } from 'react';
import Tabs from '@material-ui/core/Tabs';
import Tab from '@material-ui/core/Tab';
import FailedContent from './failedContent';
import QueueContent from './queueContent';
import Workers from './workers';

interface Props {
  queues: Array<string>,
};

const Queues: React.FC<Props> = ({ queues }) => {
  const [activeQueue, setQueue] = useState(0);
  function handleChange(event: React.ChangeEvent<{}>, newValue: number) {
    setQueue(newValue);
  }
  function renderQueues() {
    if (activeQueue >= queues.length) {
      return null;
    }
    return (<QueueContent queue={queues[activeQueue]} />);
  }
  return (
    <div>
      <Tabs value={activeQueue} onChange={handleChange}>
        {queues.map((q) => (<Tab key={q} label={q} />))}
        <Tab key="failed" label="Failed" />
        <Tab key="workers" label="Workers" />
      </Tabs>
      {renderQueues()}
      {activeQueue === queues.length && <FailedContent key="failed" />}
      {activeQueue === queues.length + 1 && <Workers key="workers" />}
    </div>
  )
}

export default Queues;
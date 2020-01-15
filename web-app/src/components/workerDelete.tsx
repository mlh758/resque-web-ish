import React, { useState } from 'react';
import IconButton from '@material-ui/core/IconButton';
import DeleteIcon from '@material-ui/icons/Delete';

interface WorkerProp {
    workerId: string,
    onDelete(id: string): void,
}

const WorkerDelete: React.FC<WorkerProp> = ({ workerId, onDelete }) => {
  const [deleting, setDeleting] = useState(false);
  const deleteWorker = () => {
    setDeleting(true);
    fetch(`worker/${encodeURIComponent(workerId)}`, { method: 'DELETE' })
    .then((response) => {
      if (response.ok) {
        onDelete(workerId);
      }
    }).finally(() => {
      setDeleting(false);
    });
  }
  return (
    <IconButton aria-label="delete" onClick={deleteWorker} disabled={deleting}>
      <DeleteIcon />
    </IconButton>
  );
}

export default WorkerDelete;
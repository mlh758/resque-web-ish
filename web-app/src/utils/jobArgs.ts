interface JobArg {
  job_class: string,
  job_id: string,
  executions: Number,
  arguments: ArgList,
}

type ArgVal = string | Number | null;
type ArgList = Array<ArgVal | Array<ArgVal>>;

export interface QueueJob {
  args: Array<JobArg>
}

const renderArguments = (args: ArgList) => {
  return args.reduce((acc, val, idx) => {
    let nextVal = "";
    if (Array.isArray(val)) {
      const cutoff = Math.min(val.length, 10);
      nextVal = `[${val.slice(0, cutoff).join(", ")}${cutoff < val.length ? "..." : ""}]`
    } else if (val === null || val === undefined) {
      nextVal = "null"
    } else {
      nextVal = val.toString();
    }
    return `${acc}${idx > 0 ? ", " : ""}${nextVal}`
  }, '') || '<none>';
};

export { renderArguments }
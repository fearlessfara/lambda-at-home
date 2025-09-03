exports.handler = async (event) => {
  console.log("echo event", event);
  return { ok: true, input: event };
};
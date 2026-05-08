export function formatNodeVariableLabel(
  nodeName: string,
  variableName: string
) {
  if (nodeName === 'sys') {
    return `sys.${variableName}`;
  }

  return `${nodeName}/${variableName}`;
}

export function formatNodeVariablePathLabel(
  nodeName: string,
  variablePath: string
) {
  return formatNodeVariableLabel(nodeName, variablePath);
}

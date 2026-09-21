export function deviceStatusKind(status) {
  if (status && typeof status === 'object' && 'Error' in status) {
    return 'Error';
  }
  return status;
}

export function deviceStatusReason(status) {
  if (status && typeof status === 'object' && typeof status.Error === 'string') {
    return status.Error;
  }
  return null;
}

export function deviceStatusLabel(status) {
  switch (deviceStatusKind(status)) {
    case 'ContinuousMode':
    case 'Running':
      return 'Connected';
    case 'Error':
      return 'Error';
    default:
      return 'Available';
  }
}

export function deviceStatusColor(status) {
  switch (deviceStatusKind(status)) {
    case 'ContinuousMode':
    case 'Running':
      return 'success';
    case 'Error':
      return 'error';
    default:
      return 'warning';
  }
}

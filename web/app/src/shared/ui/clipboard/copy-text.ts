import copy from 'copy-to-clipboard';

export async function copyTextToClipboard(text: string) {
  const successful = await copy(text);

  if (!successful) {
    throw new Error('Copy command failed');
  }
}

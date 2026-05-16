export function createBlankJsBlockTemplateCode(input: {
  blockId: string;
  codeRef: string;
  contributionCode: string;
}): string {
  return `import { defineBlock } from '@1flowbase/block-sdk';
import { Alert, Stack, Text, Title } from '@1flowbase/block-renderer/antd-facade';

// blockId: '${input.blockId}'
// codeRef: '${input.codeRef}'
// contributionCode: '${input.contributionCode}'

export default defineBlock({
  id: '${input.blockId}',
  title: 'Blank JS Block',
  initialState: {
    error: null
  },

  async render(ctx) {
    const error = typeof ctx.state.error === 'string' ? ctx.state.error : null;

    // Read records:
    // const records = await ctx.data.query('data_model_code', {
    //   page: 1,
    //   pageSize: 20
    // });

    // Create/update/delete records:
    // const created = await ctx.data.create('data_model_code', {});
    // await ctx.data.update('data_model_code', created.id, {});
    // await ctx.data.delete('data_model_code', created.id);

    return Stack({
      children: [
        Title({ children: 'Blank JS Block' }),
        Text({
          children: 'Start from this built-in blank JS Block skeleton.'
        }),
        error ? Alert({ props: { type: 'error', message: error } }) : null
      ]
    });
  }
});
`;
}

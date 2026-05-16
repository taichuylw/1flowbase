export function createBlankJsBlockTemplateCode(input: {
  blockId: string;
  codeRef: string;
  contributionCode: string;
}): string {
  return `import { defineBlock } from '@1flowbase/block-sdk';
import { Card, Space, Typography } from '@1flowbase/block-renderer/antd-facade';

export default defineBlock({
  meta: {
    blockId: '${input.blockId}',
    codeRef: '${input.codeRef}',
    contributionCode: '${input.contributionCode}'
  },

  async setup(ctx) {
    const state = ctx.state({
      title: 'Blank JS Block',
      error: null
    });

    return { state };
  },

  async render(ctx) {
    const { state } = ctx;

    // Read records:
    // const records = await ctx.data.query('data_model_code', {
    //   page: 1,
    //   pageSize: 20
    // });

    // Create/update/delete records:
    // const created = await ctx.data.create('data_model_code', {});
    // await ctx.data.update('data_model_code', created.id, {});
    // await ctx.data.delete('data_model_code', created.id);

    return Card({
      title: state.title,
      children: Space({
        direction: 'vertical',
        children: [
          Typography.Text({
            children: 'Start from this built-in blank JS Block skeleton.'
          }),
          state.error
            ? Typography.Text({ type: 'danger', children: state.error })
            : null
        ]
      })
    });
  }
});
`;
}

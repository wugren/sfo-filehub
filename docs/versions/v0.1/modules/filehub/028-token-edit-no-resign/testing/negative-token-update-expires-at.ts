// 负面契约夹具（028）：`TokenUpdateInput.expires_at` 已按提案移除，
// 引用该字段的调用必须编译失败；wrapper 只接受「因 expires_at 报错」的失败。
import type { TokenUpdateInput } from "../../../../../../../admin-web/src/api/contract";

const patch: TokenUpdateInput = {
  name: "negative",
  expires_at: null,
};

void patch;

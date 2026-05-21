/* 004258e1 FUN_004258e1 */

void __thiscall FUN_004258e1(void *this,int param_1,int param_2,int param_3)

{
  undefined4 uVar1;
  
  uVar1 = *(undefined4 *)(param_1 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c + param_2 * 4);
  *(undefined4 *)(param_1 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c + param_2 * 4) =
       *(undefined4 *)(param_1 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c + param_3 * 4);
  *(undefined4 *)(param_1 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c + param_3 * 4) = uVar1;
  *(int *)(*(int *)((int)this + 0x3a4) + 0xf4 + param_1 * 0xfc) = param_2;
  *(int *)(*(int *)((int)this + 0x3a4) + 0xf8 + param_1 * 0xfc) = param_3;
  return;
}

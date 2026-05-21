/* 00437348 FUN_00437348 */

void __fastcall FUN_00437348(int param_1)

{
  undefined4 local_c;
  undefined4 local_8;
  
  for (local_8 = *(int *)(param_1 + 0x1654); local_8 < *(int *)(param_1 + 0x1650);
      local_8 = local_8 + 1) {
    FUN_0041770d((int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4)));
  }
  for (local_c = *(int *)(param_1 + 0x1658); local_c < *(int *)(param_1 + 0x1650);
      local_c = local_c + 1) {
    if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_c * 0xfc) == 0) {
      FUN_00417769((void *)(local_c * 0xfc + *(int *)(param_1 + 0x3a4)),
                   *(undefined4 *)(param_1 + 0x3a4),0);
    }
  }
  return;
}

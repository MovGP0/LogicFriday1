/* 0043c8a6 FUN_0043c8a6 */

undefined4 __thiscall FUN_0043c8a6(void *this,HWND param_1)

{
  undefined4 uVar1;
  undefined4 *puVar2;
  int iVar3;
  int iVar4;
  int *local_c;
  int local_8;
  
  local_8 = 0;
  while( true ) {
    if (*(int *)((int)this + 0x30) <= local_8) {
      return 0;
    }
    SendMessageA(param_1,0x800d,(WPARAM)&local_c,
                 *(LPARAM *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14));
    iVar3 = FUN_0043c96e(this,(int)local_c);
    if (iVar3 != 0) break;
    local_8 = local_8 + 1;
  }
  if (*(int *)this == -3) {
    uVar1 = *(undefined4 *)(*(int *)((int)this + 0x34) + 4 + local_8 * 0x14);
    puVar2 = *(undefined4 **)((int)this + 0x2c);
    *puVar2 = *(undefined4 *)(*(int *)((int)this + 0x34) + local_8 * 0x14);
    puVar2[1] = uVar1;
  }
  else {
    uVar1 = *(undefined4 *)(*(int *)((int)this + 0x34) + 4 + local_8 * 0x14);
    iVar4 = (*(int *)((int)this + 0x28) + -1) * 0x14;
    iVar3 = *(int *)((int)this + 0x2c);
    *(undefined4 *)(iVar3 + iVar4) = *(undefined4 *)(*(int *)((int)this + 0x34) + local_8 * 0x14);
    *(undefined4 *)(iVar3 + 4 + iVar4) = uVar1;
  }
  FUN_0043cc09(this,local_c,param_1);
  return 1;
}

/* 0043c96e FUN_0043c96e */

undefined4 __thiscall FUN_0043c96e(void *this,int param_1)

{
  POINT pt;
  POINT pt_00;
  BOOL BVar1;
  int iVar2;
  tagRECT local_14;
  
  if (*(int *)((int)this + 0x3c) != 0) {
    if (*(int *)this == -3) {
      local_14.left = **(LONG **)((int)this + 0x2c);
      local_14.top = *(LONG *)(*(int *)((int)this + 0x2c) + 4);
      local_14.right = local_14.left;
      local_14.bottom = local_14.top;
      InflateRect(&local_14,6,6);
      BVar1 = PtInRect(&local_14,**(POINT **)(param_1 + 0x2c));
      if ((BVar1 != 0) ||
         (iVar2 = (*(int *)(param_1 + 0x28) + -1) * 0x14,
         pt.y = *(LONG *)(*(int *)(param_1 + 0x2c) + 4 + iVar2),
         pt.x = *(LONG *)(*(int *)(param_1 + 0x2c) + iVar2), BVar1 = PtInRect(&local_14,pt),
         BVar1 != 0)) {
        return 1;
      }
    }
    else {
      local_14.left =
           *(LONG *)((*(int *)((int)this + 0x28) + -1) * 0x14 + *(int *)((int)this + 0x2c));
      local_14.top = *(LONG *)(*(int *)((int)this + 0x2c) + 4 +
                              (*(int *)((int)this + 0x28) + -1) * 0x14);
      local_14.right = local_14.left;
      local_14.bottom = local_14.top;
      InflateRect(&local_14,6,6);
      BVar1 = PtInRect(&local_14,**(POINT **)(param_1 + 0x2c));
      if ((BVar1 != 0) ||
         (iVar2 = (*(int *)(param_1 + 0x28) + -1) * 0x14,
         pt_00.y = *(LONG *)(*(int *)(param_1 + 0x2c) + 4 + iVar2),
         pt_00.x = *(LONG *)(*(int *)(param_1 + 0x2c) + iVar2), BVar1 = PtInRect(&local_14,pt_00),
         BVar1 != 0)) {
        return 1;
      }
    }
  }
  return 0;
}

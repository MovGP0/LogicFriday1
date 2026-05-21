/* 0043147c FUN_0043147c */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

uint __thiscall
FUN_0043147c(void *this,undefined4 param_1,LONG param_2,LONG param_3,undefined4 *param_4,int param_5
            )

{
  int iVar1;
  POINT pt;
  BOOL BVar2;
  uint unaff_retaddr;
  char local_124 [256];
  uint local_24;
  uint local_20;
  tagRECT local_1c;
  int local_c;
  int local_8;
  
  local_24 = DAT_00451a00 ^ unaff_retaddr;
  local_c = 64000;
  local_20 = 0;
  for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c4); local_8 = local_8 + 1) {
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0x48) == 0) {
      iVar1 = *(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4);
      local_1c.left = *(LONG *)(iVar1 + 200);
      local_1c.top = *(LONG *)(iVar1 + 0xcc);
      local_1c.right = *(LONG *)(iVar1 + 0xd0);
      local_1c.bottom = *(LONG *)(iVar1 + 0xd4);
      InflateRect(&local_1c,-3,-3);
      pt.y = param_3;
      pt.x = param_2;
      BVar2 = PtInRect(&local_1c,pt);
      if (BVar2 == 0) {
        if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xd8) != 0) {
          if (param_5 == 0) {
            *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xd8) = 0;
          }
          else {
            local_20 = local_20 + 1;
          }
        }
      }
      else if (local_c == 64000) {
        *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xd8) = 1;
        iVar1 = *(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4);
        *param_4 = *(undefined4 *)(iVar1 + 200);
        param_4[1] = *(undefined4 *)(iVar1 + 0xcc);
        param_4[2] = *(undefined4 *)(iVar1 + 0xd0);
        param_4[3] = *(undefined4 *)(iVar1 + 0xd4);
        local_c = local_8;
        local_20 = local_20 + 1;
      }
    }
  }
  if ((DAT_00452ef4 != 0) && (local_c != 64000)) {
    FUN_0043ed39(local_124,(byte *)"Gate %d: iOutWire=%d, iInWire[0]=%d, iInWire[1]=%d");
    FUN_0040bdc3((LPARAM)local_124);
  }
  return local_20 & 0xffff | local_c << 0x10;
}

/* 004194bc FUN_004194bc */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_004194bc(void *this,char *param_1)

{
  undefined4 uVar1;
  int iVar2;
  uint unaff_retaddr;
  undefined1 local_74 [8];
  undefined4 local_6c;
  char *local_60;
  undefined4 local_5c;
  undefined1 local_4c [8];
  undefined4 local_44;
  char *local_38;
  undefined4 local_34;
  int local_24;
  undefined4 local_20;
  WPARAM local_1c;
  char local_18 [12];
  uint local_c;
  size_t local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  local_20 = 0;
  local_8 = _strlen(param_1);
  if ((local_8 == 0) || (8 < local_8)) {
    uVar1 = 1;
  }
  else {
    local_24 = SendMessageA(*(HWND *)((int)this + 0x18),0x1004,0,0);
    for (local_1c = 0; (int)local_1c < local_24; local_1c = local_1c + 1) {
      local_44 = 0;
      local_34 = 9;
      local_38 = local_18;
      SendMessageA(*(HWND *)((int)this + 0x18),0x102d,local_1c,(LPARAM)local_4c);
      iVar2 = _strcmp(local_18,param_1);
      if (iVar2 == 0) {
        return 2;
      }
    }
    local_24 = SendMessageA(*(HWND *)((int)this + 0x1c),0x1004,0,0);
    for (local_1c = 0; (int)local_1c < local_24; local_1c = local_1c + 1) {
      local_6c = 0;
      local_5c = 9;
      local_60 = local_18;
      SendMessageA(*(HWND *)((int)this + 0x1c),0x102d,local_1c,(LPARAM)local_74);
      iVar2 = _strcmp(local_18,param_1);
      if (iVar2 == 0) {
        return 2;
      }
    }
    uVar1 = FUN_0040daf0(param_1);
  }
  return uVar1;
}

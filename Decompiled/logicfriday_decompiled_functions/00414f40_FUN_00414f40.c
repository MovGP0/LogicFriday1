/* 00414f40 FUN_00414f40 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int __thiscall FUN_00414f40(void *this,int param_1)

{
  int iVar1;
  uint unaff_retaddr;
  char local_41c;
  char local_41b;
  char local_419 [1025];
  uint local_18;
  int local_14;
  int local_10;
  FILE *local_c;
  int local_8;
  
  local_18 = DAT_00451a00 ^ unaff_retaddr;
  local_8 = 0;
  local_c = (FILE *)FUN_0043e6f2((char *)((int)this + 0x888),"rt");
  if (local_c == (FILE *)0x0) {
    local_10 = 0x2f000e;
  }
  else {
    FUN_0043f99d(&local_41c,0x400,local_c);
    if (local_41c == '.') {
      while (local_41c == '.') {
        if (local_41b == 'p') {
          local_14 = _atol(local_419);
          break;
        }
        FUN_0043f99d(&local_41c,0x400,local_c);
      }
      local_10 = FUN_00421c38(*(void **)(param_1 + 4),local_14);
      if (local_10 == 0) {
        while ((local_c->_flag & 0x10U) == 0) {
          FUN_0043f99d(&local_41c,0x164,local_c);
          iVar1 = __strnicmp(&local_41c,".e",2);
          if (iVar1 == 0) break;
          FUN_00421d2a(*(void **)(param_1 + 4),local_8,(int)&local_41c);
          local_8 = local_8 + 1;
        }
        _fclose(local_c);
        local_10 = 0;
      }
      else {
        _fclose(local_c);
      }
    }
    else {
      _fclose(local_c);
      local_10 = 0x1d0004;
    }
  }
  return local_10;
}

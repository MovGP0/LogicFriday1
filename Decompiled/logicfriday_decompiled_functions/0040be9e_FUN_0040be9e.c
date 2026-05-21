/* 0040be9e FUN_0040be9e */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int __cdecl FUN_0040be9e(char *param_1)

{
  bool bVar1;
  size_t sVar2;
  int iVar3;
  long lVar4;
  uint unaff_retaddr;
  int local_2c;
  int local_28;
  char local_24 [24];
  uint local_c;
  char *local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  local_2c = 0;
  bVar1 = false;
  FUN_0043ed39(local_24,&DAT_0044b738);
  for (local_28 = 0; local_28 < DAT_004528a0; local_28 = local_28 + 1) {
    sVar2 = _strlen(local_24);
    iVar3 = __strnicmp((char *)(DAT_004528a4 + local_28 * 0x118),local_24,sVar2);
    if (iVar3 == 0) {
      iVar3 = DAT_004528a4 + local_28 * 0x118;
      sVar2 = _strlen(local_24);
      local_8 = (char *)(iVar3 + sVar2);
      lVar4 = _atol(local_8);
      if (local_2c < lVar4) {
        local_2c = lVar4;
      }
    }
  }
  if (local_2c < 1) {
    for (local_28 = 0; local_28 < DAT_004528a0; local_28 = local_28 + 1) {
      iVar3 = __stricmp((char *)(DAT_004528a4 + local_28 * 0x118),param_1);
      if (iVar3 == 0) {
        bVar1 = true;
        break;
      }
    }
    if (bVar1) {
      local_2c = 2;
    }
    else {
      local_2c = 0;
    }
  }
  else {
    local_2c = local_2c + 1;
  }
  return local_2c;
}

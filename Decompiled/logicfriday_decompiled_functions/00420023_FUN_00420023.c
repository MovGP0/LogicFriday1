/* 00420023 FUN_00420023 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int __thiscall FUN_00420023(void *this,char *param_1,int param_2)

{
  char cVar1;
  int iVar2;
  long lVar3;
  uint unaff_retaddr;
  int local_11c;
  int local_114;
  char *local_110;
  char local_10c [260];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  local_110 = param_1;
  local_11c = 0;
  do {
    if (*local_110 == '{') {
      local_114 = 0;
      while ((local_110 = local_110 + 1, *local_110 != '}' && (*local_110 != ','))) {
        local_10c[local_114] = *local_110;
        local_114 = local_114 + 1;
      }
      local_10c[local_114] = '\0';
      for (local_114 = *(int *)((int)this + 0x1654); local_114 < *(int *)((int)this + 0x1650);
          local_114 = local_114 + 1) {
        iVar2 = _strcmp((char *)(*(int *)((int)this + 0x3a4) + 4 + local_114 * 0xfc),local_10c);
        if (iVar2 == 0) {
          *(int *)(*(int *)((int)this + 0x3a4) + 0x3c + local_114 * 0xfc) = param_2;
          local_11c = local_11c + 1;
          break;
        }
      }
    }
    else if (*local_110 == '[') {
      local_110 = local_110 + 1;
      lVar3 = _atol(local_110);
      if (*(int *)(*(int *)((int)this + 0x3a4) + 0x48 +
                  (lVar3 + *(int *)((int)this + 0x1654)) * 0xfc) == 0) {
        *(int *)(*(int *)((int)this + 0x3a4) + 0x3c + (lVar3 + *(int *)((int)this + 0x1654)) * 0xfc)
             = param_2;
        local_11c = local_11c + 1;
      }
      do {
        cVar1 = *local_110;
        local_110 = local_110 + 1;
      } while (cVar1 != ']');
    }
    else {
      local_110 = local_110 + 1;
    }
    if (*local_110 == '\0') {
      *(int *)(*(int *)((int)this + 0x1668) + param_2 * 4) = local_11c;
      return local_11c;
    }
  } while( true );
}

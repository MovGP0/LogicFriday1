/* 0040be0b FUN_0040be0b */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int FUN_0040be0b(void)

{
  bool bVar1;
  int iVar2;
  uint unaff_retaddr;
  char local_2c [32];
  uint local_c;
  int local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  do {
    bVar1 = false;
    FUN_0043ed39(local_2c,&DAT_0044a700);
    for (local_8 = 0; local_8 < DAT_004528a0; local_8 = local_8 + 1) {
      iVar2 = __stricmp((char *)(DAT_004528a4 + local_8 * 0x118),local_2c);
      if (iVar2 == 0) {
        bVar1 = true;
        DAT_0046c504 = DAT_0046c504 + 1;
        break;
      }
    }
    if (!bVar1) {
      return DAT_0046c504;
    }
  } while( true );
}

/* 0040b894 FUN_0040b894 */

int __cdecl FUN_0040b894(char *param_1)

{
  bool bVar1;
  int iVar2;
  undefined4 local_c;
  
  bVar1 = false;
  local_c = 0;
  do {
    if (DAT_004528a0 <= local_c) {
LAB_0040b8e0:
      if (!bVar1) {
        local_c = -1;
      }
      return local_c;
    }
    iVar2 = __stricmp((char *)(DAT_004528a4 + local_c * 0x118),param_1);
    if (iVar2 == 0) {
      bVar1 = true;
      goto LAB_0040b8e0;
    }
    local_c = local_c + 1;
  } while( true );
}

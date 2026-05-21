/* 0040b8f0 FUN_0040b8f0 */

int __cdecl FUN_0040b8f0(int param_1)

{
  bool bVar1;
  undefined4 local_c;
  
  bVar1 = false;
  local_c = 0;
  do {
    if (DAT_004528a0 <= local_c) {
LAB_0040b937:
      if (!bVar1) {
        local_c = -1;
      }
      return local_c;
    }
    if (param_1 == *(int *)(DAT_004528a4 + 0x110 + local_c * 0x118)) {
      bVar1 = true;
      goto LAB_0040b937;
    }
    local_c = local_c + 1;
  } while( true );
}

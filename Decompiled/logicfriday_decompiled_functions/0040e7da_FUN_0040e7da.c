/* 0040e7da FUN_0040e7da */

undefined4 __cdecl FUN_0040e7da(int param_1,int param_2)

{
  bool bVar1;
  undefined4 uVar2;
  char *pcVar3;
  int iVar4;
  int local_10;
  int local_c;
  
  bVar1 = true;
  if (*(int *)(param_1 + 0xc4) == *(int *)(param_2 + 0xc4)) {
    pcVar3 = _strstr(*(char **)(param_2 + 0x268),"INORDER");
    if (pcVar3 == (char *)0x0) {
      for (local_c = 0; local_c < *(int *)(param_1 + 0xc4); local_c = local_c + 1) {
        iVar4 = _strcmp((char *)(param_1 + 0x160 + local_c * 9),
                        (char *)(param_2 + 0x160 + local_c * 9));
        if (iVar4 != 0) {
          bVar1 = false;
          break;
        }
      }
      if (bVar1) {
        uVar2 = 1;
      }
      else {
        for (local_c = 0; local_c < *(int *)(param_1 + 0xc4); local_c = local_c + 1) {
          bVar1 = false;
          local_10 = 0;
          while (local_c < *(int *)(param_2 + 0xc4)) {
            iVar4 = _strcmp((char *)(param_1 + 0x160 + local_c * 9),
                            (char *)(param_2 + 0x160 + local_10 * 9));
            if (iVar4 == 0) {
              bVar1 = true;
              break;
            }
            local_10 = local_10 + 1;
          }
        }
        if (bVar1) {
          uVar2 = 0;
        }
        else {
          uVar2 = 1;
        }
      }
    }
    else {
      uVar2 = 1;
    }
  }
  else {
    uVar2 = 1;
  }
  return uVar2;
}

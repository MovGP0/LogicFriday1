/* 0040e1e6 FUN_0040e1e6 */

undefined4 __cdecl FUN_0040e1e6(int param_1,short param_2)

{
  int iVar1;
  int iVar2;
  undefined4 uVar3;
  int local_c;
  
  iVar1 = *(int *)(param_1 + 0xc4);
  iVar2 = *(int *)(param_1 + 200);
  if (((param_2 == 0xe9) && (9 < iVar1)) && (100 < *(uint *)(param_1 + 4))) {
    uVar3 = 1;
  }
  else if (iVar1 < 0xb) {
    uVar3 = 0;
  }
  else if (iVar1 == 0xb) {
    if (iVar2 < 5) {
      uVar3 = 0;
    }
    else {
      for (local_c = 0; local_c < iVar2; local_c = local_c + 1) {
        if (0x266 < *(uint *)(param_1 + 4 + local_c * 4)) {
          return 1;
        }
      }
      uVar3 = 0;
    }
  }
  else if (iVar1 == 0xc) {
    if (iVar2 < 3) {
      uVar3 = 0;
    }
    else {
      for (local_c = 0; local_c < iVar2; local_c = local_c + 1) {
        if (0x4cc < *(uint *)(param_1 + 4 + local_c * 4)) {
          return 1;
        }
      }
      uVar3 = 0;
    }
  }
  else if (iVar1 == 0xd) {
    if (iVar2 < 2) {
      uVar3 = 0;
    }
    else {
      for (local_c = 0; local_c < iVar2; local_c = local_c + 1) {
        if (0x998 < *(uint *)(param_1 + 4 + local_c * 4)) {
          return 1;
        }
      }
      uVar3 = 0;
    }
  }
  else if (iVar1 == 0xe) {
    for (local_c = 0; local_c < iVar2; local_c = local_c + 1) {
      if (0x1330 < *(uint *)(param_1 + 4 + local_c * 4)) {
        return 1;
      }
    }
    uVar3 = 0;
  }
  else if (iVar1 == 0xf) {
    for (local_c = 0; local_c < iVar2; local_c = local_c + 1) {
      if (0x1330 < *(uint *)(param_1 + 4 + local_c * 4)) {
        return 1;
      }
    }
    uVar3 = 0;
  }
  else if (iVar1 == 0x10) {
    for (local_c = 0; local_c < iVar2; local_c = local_c + 1) {
      if (0x1330 < *(uint *)(param_1 + 4 + local_c * 4)) {
        return 1;
      }
    }
    uVar3 = 0;
  }
  else {
    uVar3 = 0;
  }
  return uVar3;
}

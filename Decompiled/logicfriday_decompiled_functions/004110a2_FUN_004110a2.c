/* 004110a2 FUN_004110a2 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 FUN_004110a2(HWND param_1,uint *param_2,char *param_3)

{
  undefined4 uVar1;
  char *pcVar2;
  void *pvVar3;
  uint unaff_retaddr;
  uint local_334 [90];
  int local_1cc;
  uint local_1c8 [7];
  uint local_1ac [7];
  size_t local_190;
  int local_18c;
  int local_188;
  uint local_184;
  FILE *local_180;
  uint local_17c;
  size_t local_178;
  uint local_174 [90];
  uint local_c;
  undefined4 local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  local_8 = 0;
  local_190 = 0;
  local_178 = 0;
  local_18c = 0;
  local_188 = 0;
  local_180 = (FILE *)FUN_0043e6f2(param_3,"rt");
  if (local_180 == (FILE *)0x0) {
    uVar1 = 0x2b0004;
  }
  else {
    FUN_0043ebd0(local_334,(uint *)&DAT_0044ad26);
    FUN_0043ebd0(local_174,(uint *)&DAT_0044ad26);
    FUN_0043ebd0(local_1ac,(uint *)&DAT_0044ad26);
    FUN_0043ebd0(local_1c8,(uint *)&DAT_0044ad26);
    do {
      if (((local_180->_flag & 0x10U) != 0) ||
         (pcVar2 = FUN_0043f99d((char *)local_334,0x164,local_180), pcVar2 == (char *)0x0))
      goto LAB_004111bf;
      local_188 = local_188 + 1;
    } while (((char)local_334[0] == '%') ||
            (((char)local_334[0] == '\n' || ((char)local_334[0] == '\r'))));
    local_18c = 1;
LAB_004111bf:
    if (local_18c == 0) {
      _fclose(local_180);
      FUN_00411f69(param_1,0x70000);
      uVar1 = 0x1e240;
    }
    else {
      FUN_0043ebd0(local_174,local_334);
      pcVar2 = FUN_0043f99d((char *)local_334,0x400,local_180);
      if (pcVar2 == (char *)0x0) {
        _fclose(local_180);
        FUN_00411f69(param_1,0x70000);
        uVar1 = 0x1e240;
      }
      else {
        local_188 = local_188 + 1;
        local_184 = FUN_004116b2(local_334,local_1ac,local_1c8);
        if (local_184 == 0) {
          local_190 = _strlen((char *)local_1ac);
          local_178 = _strlen((char *)local_1c8);
          if (((((int)local_190 < 2) || (0x10 < (int)local_190)) || ((int)local_178 < 1)) ||
             (0x10 < (int)local_178)) {
            _fclose(local_180);
            FUN_00411f69(param_1,0x80000);
            uVar1 = 0x1e240;
          }
          else {
            param_2[0x31] = local_190;
            param_2[0x32] = local_178;
            local_184 = FUN_00411871(local_174,(int)param_2);
            if (local_184 == 0) {
              *param_2 = 1 << ((byte)local_190 & 0x1f);
              local_184 = 0;
              for (local_17c = 0; (int)local_17c < (int)local_178; local_17c = local_17c + 1) {
                pvVar3 = _realloc((void *)param_2[local_17c + 0x21],*param_2 << 2);
                param_2[local_17c + 0x21] = (uint)pvVar3;
                if (param_2[local_17c + 0x21] == 0) {
                  _fclose(local_180);
                  return 0x4000c;
                }
                _memset((void *)param_2[local_17c + 0x21],0xff,*param_2 << 2);
              }
              local_184 = FUN_00411caa(param_2,(char *)local_1ac,(int)local_1c8);
              if (local_184 == 0) {
                do {
                  do {
                    if ((local_180->_flag & 0x10U) != 0) {
LAB_0041161b:
                      for (local_17c = 0; local_17c < *param_2; local_17c = local_17c + 1) {
                        if (*(int *)(param_2[0x21] + local_17c * 4) == -1) {
                          for (local_1cc = 0; local_1cc < (int)local_178; local_1cc = local_1cc + 1)
                          {
                            *(undefined4 *)(param_2[local_1cc + 0x21] + local_17c * 4) = 0;
                          }
                        }
                      }
                      _fclose(local_180);
                      return 0;
                    }
                    pcVar2 = FUN_0043f99d((char *)local_334,0x400,local_180);
                  } while (pcVar2 == (char *)0x0);
                  local_188 = local_188 + 1;
                  if ((((char)local_334[0] == '%') || ((char)local_334[0] == '\n')) ||
                     ((char)local_334[0] == '\r')) goto LAB_0041161b;
                  local_184 = FUN_004116b2(local_334,local_1ac,local_1c8);
                  if (local_184 != 0) {
                    FUN_00411f69(param_1,local_184 * 0x10000 + local_188);
                    _fclose(local_180);
                    return 0x1e240;
                  }
                  local_184 = FUN_00411caa(param_2,(char *)local_1ac,(int)local_1c8);
                } while (local_184 == 0);
                FUN_00411f69(param_1,local_184 * 0x10000 + local_188);
                _fclose(local_180);
                uVar1 = 0x1e240;
              }
              else {
                FUN_00411f69(param_1,local_184 * 0x10000 + local_188);
                _fclose(local_180);
                uVar1 = 0x1e240;
              }
            }
            else {
              if (local_184 >> 0x10 == 6) {
                FUN_00411f69(param_1,local_184);
              }
              else {
                FUN_00411f69(param_1,local_184 * 0x10000 + local_188);
              }
              _fclose(local_180);
              uVar1 = 0x1e240;
            }
          }
        }
        else {
          FUN_00411f69(param_1,local_184 * 0x10000 + local_188);
          _fclose(local_180);
          uVar1 = 0x1e240;
        }
      }
    }
  }
  return uVar1;
}

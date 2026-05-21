/* 0040daf0 FUN_0040daf0 */

undefined4 __cdecl FUN_0040daf0(char *param_1)

{
  char *pcVar1;
  undefined4 local_8;
  
  local_8 = 0;
  pcVar1 = _strchr(param_1,0x40);
  if (pcVar1 == (char *)0x0) {
    pcVar1 = _strchr(param_1,0x23);
    if (pcVar1 == (char *)0x0) {
      pcVar1 = _strchr(param_1,0x22);
      if (pcVar1 == (char *)0x0) {
        pcVar1 = _strchr(param_1,0x5c);
        if (pcVar1 == (char *)0x0) {
          pcVar1 = _strchr(param_1,0x7b);
          if (pcVar1 == (char *)0x0) {
            pcVar1 = _strchr(param_1,0x7d);
            if (pcVar1 == (char *)0x0) {
              pcVar1 = _strchr(param_1,0x26);
              if (pcVar1 == (char *)0x0) {
                pcVar1 = _strchr(param_1,0x2a);
                if (pcVar1 == (char *)0x0) {
                  pcVar1 = _strchr(param_1,0x7c);
                  if (pcVar1 == (char *)0x0) {
                    pcVar1 = _strchr(param_1,0x2b);
                    if (pcVar1 == (char *)0x0) {
                      pcVar1 = _strchr(param_1,0x21);
                      if (pcVar1 == (char *)0x0) {
                        pcVar1 = _strchr(param_1,0x5e);
                        if (pcVar1 == (char *)0x0) {
                          pcVar1 = _strchr(param_1,0x3d);
                          if (pcVar1 == (char *)0x0) {
                            pcVar1 = _strchr(param_1,0x2c);
                            if (pcVar1 != (char *)0x0) {
                              local_8 = 0x1f002c;
                            }
                          }
                          else {
                            local_8 = 0x1f003d;
                          }
                        }
                        else {
                          local_8 = 0x1f005e;
                        }
                      }
                      else {
                        local_8 = 0x1f0021;
                      }
                    }
                    else {
                      local_8 = 0x1f002b;
                    }
                  }
                  else {
                    local_8 = 0x1f007c;
                  }
                }
                else {
                  local_8 = 0x1f002a;
                }
              }
              else {
                local_8 = 0x1f0026;
              }
            }
            else {
              local_8 = 0x1f007d;
            }
          }
          else {
            local_8 = 0x1f007b;
          }
        }
        else {
          local_8 = 0x1f005c;
        }
      }
      else {
        local_8 = 0x1f0022;
      }
    }
    else {
      local_8 = 0x1f0023;
    }
  }
  else {
    local_8 = 0x1f0040;
  }
  return local_8;
}
